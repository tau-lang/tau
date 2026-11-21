use inkwell::{
    AddressSpace, IntPredicate, OptimizationLevel,
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::Module,
    passes::PassBuilderOptions,
    targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine},
    types::{BasicType, BasicTypeEnum},
    values::{BasicValue, BasicValueEnum, FunctionValue, PointerValue},
};
use std::{collections::HashMap, path::Path};
use symbol_table::GlobalSymbol;
use tau_core::{
    ast::{
        TypedAst,
        declaration::{DeclFolder, DeclVisitor},
        fold_vec,
    },
    error::DynResult,
    type_::{Type, TypeVar, Typed},
};

mod declaration;
mod expression;
mod statement;

pub struct LlvmCodeGen<'a, 'ctx> {
    pub context: &'ctx Context,
    pub module: &'a Module<'ctx>,
    pub builder: Builder<'ctx>,

    machine: TargetMachine,
    variables: HashMap<GlobalSymbol, PointerValue<'ctx>>,
    structs: HashMap<GlobalSymbol, HashMap<GlobalSymbol, u32>>,
    fn_value: Option<FunctionValue<'ctx>>,
    /// Stack of all scopes. Each scope tracks if it has already returned.
    scopes: Vec<bool>,
    /// Stack of all loop exit labels.
    loops: Vec<BasicBlock<'ctx>>,
}

impl<'a, 'ctx> LlvmCodeGen<'a, 'ctx> {
    /// Creates a new instance from the given llvm context and module.
    pub fn new(context: &'ctx Context, module: &'a Module<'ctx>) -> Self {
        Self {
            context,
            module,
            builder: context.create_builder(),
            machine: {
                Target::initialize_all(&InitializationConfig::default());

                // TODO: allow the cli to pass the target triple
                let triple = TargetMachine::get_default_triple();
                let target = Target::from_triple(&triple).unwrap();

                target
                    .create_target_machine(
                        &triple,
                        "generic",
                        "",
                        OptimizationLevel::Default,
                        RelocMode::PIC,
                        CodeModel::Default,
                    )
                    .unwrap()
            },
            variables: HashMap::new(),
            structs: HashMap::new(),
            fn_value: None,
            scopes: Vec::new(),
            loops: Vec::new(),
        }
    }

    /// Optimize llvm for the default triple.
    pub fn run_passes(&self) {
        let passes: &[&str] = &["reassociate", "gvn", "simplifycfg", "mem2reg"];

        self.module
            .run_passes(
                passes.join(",").as_str(),
                &self.machine,
                PassBuilderOptions::create(),
            )
            .unwrap();
    }

    /// Write the compiled object file to the given path.
    pub fn write_object_file(&self, path: &Path) -> DynResult<()> {
        self.machine
            .write_to_file(self.module, FileType::Object, path)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Folds the ast into llvm ir.
    pub fn fold_ast(&mut self, ast: TypedAst) -> DynResult<()> {
        for declaration in &ast {
            self.visit_decl(declaration)?;
        }
        fold_vec(ast, |declaration| self.fold_decl(declaration))?;

        self.module.verify()?;
        Ok(())
    }

    /// Folds the type into an inkwell basic type.
    fn fold_type(&self, type_: &Typed) -> BasicTypeEnum<'ctx> {
        match type_.as_ref() {
            Type::Named { name, .. } => match name.as_str() {
                "bool" => self.context.bool_type().as_basic_type_enum(),
                "i8" | "u8" => self.context.i8_type().as_basic_type_enum(),
                "i16" | "u16" => self.context.i16_type().as_basic_type_enum(),
                "i32" | "u32" => self.context.i32_type().as_basic_type_enum(),
                "i64" | "u64" => self.context.i64_type().as_basic_type_enum(),
                "f32" => self.context.f32_type().as_basic_type_enum(),
                "f64" => self.context.f64_type().as_basic_type_enum(),
                "usize" => self
                    .context
                    .ptr_sized_int_type(&self.machine.get_target_data(), None)
                    .as_basic_type_enum(),
                // pointer to a string slice
                "str" => {
                    Context::ptr_type(self.context, AddressSpace::default()).as_basic_type_enum()
                }
                "void" => panic!("void is not a basic type"),
                name => self
                    .module
                    .get_struct_type(name)
                    .unwrap_or_else(|| panic!("struct {name} exists"))
                    .as_basic_type_enum(),
            },
            Type::Var { type_: lock } => match lock.try_read() {
                Ok(type_var) => match &*type_var {
                    TypeVar::Unbound { .. } | TypeVar::Generic { .. } => {
                        panic!("type must be resolved")
                    }
                    TypeVar::Link { type_ } => self.fold_type(type_),
                },
                Err(_) => unreachable!(),
            },
            Type::Array { type_, size } => self
                .fold_type(type_)
                .array_type(*size as u32)
                .as_basic_type_enum(),
            Type::Ptr { .. } => Context::ptr_type(self.context, AddressSpace::default()).into(),
            Type::Fn { .. } => panic!("functions are not basic types"),
        }
    }

    /// Build a new stack allocation instruction in the entry block of the
    /// function.
    fn build_entry_block_alloca(&self, name: &str, type_: &Typed) -> PointerValue<'ctx> {
        let builder = self.context.create_builder();

        let entry = self.fn_value().get_first_basic_block().unwrap();

        match entry.get_first_instruction() {
            Some(first_instr) => builder.position_before(&first_instr),
            None => builder.position_at_end(entry),
        }

        let type_ = self.fold_type(type_);
        builder.build_alloca(type_, name).unwrap()
    }

    /// Convert a language boolean into LLVM's i1 condition.
    fn as_condition(&self, value: BasicValueEnum<'ctx>) -> inkwell::values::IntValue<'ctx> {
        let value = value.into_int_value();

        self.builder
            .build_int_compare(
                IntPredicate::NE,
                value,
                value.get_type().const_zero(),
                "boolcond",
            )
            .unwrap()
    }

    /// Convert LLVM i1 into the language's i8 boolean representation.
    fn as_bool(&self, value: inkwell::values::IntValue<'ctx>) -> BasicValueEnum<'ctx> {
        self.builder
            .build_int_z_extend(value, self.context.i8_type(), "bool")
            .unwrap()
            .as_basic_value_enum()
    }

    /// Gets a defined function given its name.
    #[inline]
    fn get_function(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        self.module.get_function(name)
    }

    /// Returns the `FunctionValue` representing the function being compiled.
    #[inline]
    fn fn_value(&self) -> FunctionValue<'ctx> {
        self.fn_value.unwrap()
    }

    fn begin_scope(&mut self) {
        self.scopes.push(false);
    }

    fn end_scope(&mut self) {
        self.scopes.pop().expect("scopes not empty");
    }

    fn mark_early_return(&mut self) {
        *self.scopes.last_mut().expect("scopes not empty") = true;
    }

    fn has_returned(&self) -> bool {
        *self.scopes.last().expect("scopes not empty")
    }

    fn begin_loop(&mut self, label: BasicBlock<'ctx>) {
        self.loops.push(label);
    }

    fn end_loop(&mut self) {
        self.loops.pop();
    }

    fn loop_label(&self) -> &BasicBlock<'ctx> {
        self.loops.last().expect("loop exists")
    }
}
