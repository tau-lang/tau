//! Emits QBE code.
//!
//! This generator creates a `.ssa` file. You must currently generate the
//! assembly code yourself by invoking the QBE binary and link it with the
//! linker of your choice:
//!
//! ```sh
//! qbe -o main.s main.ssa && cc -o main main.s
//! ```

use qbe::{Block, DataDef, Function, Module, TypeDef};
use std::{collections::HashMap, mem, sync::Arc};
use symbol_table::{GlobalSymbol, static_symbol};
use tau_core::{
    ast::Unfold,
    ir::{
        aggregate::Aggregate,
        data::DataItem,
        emitter::Emitter,
        instruction::Cmp,
        linkage::Linkage,
        value::{Value, ValueKind},
    },
    type_::{self, Typed},
};

/// Creates a new empty QBE function. This function has an empty name. Please
/// note that empty names are not valid and the name must be set before the
/// function ends.
const fn function() -> Function {
    Function {
        linkage: qbe::Linkage {
            exported: false,
            section: None,
            secflags: None,
            thread_local: false,
        },
        name: String::new(),
        arguments: Vec::new(),
        blocks: Vec::new(),
        return_ty: None,
    }
}

/// Creates a new empty QBE block. This block has an empty name. Please note
/// that empty names are not valid and the name must be set before the block
/// ends.
const fn block() -> Block {
    Block {
        label: String::new(),
        items: Vec::new(),
    }
}

/// Code emitter for QBE. Each emitter only emits a single module. Using the
/// same emitter to fold multiple Tau modules is undefined behavior.
#[derive(Default)]
pub struct QbeEmitter {
    module: Module,
    function: Function,
    block: Block,

    cache: HashMap<GlobalSymbol, qbe::Type>,
}

impl QbeEmitter {
    /// Creates a new emitter and loads the build in types. If you don't want to
    /// load build in types automatically, you can use default to create an
    /// empty emitter.
    pub fn new() -> Self {
        Self {
            cache: {
                let mut cache = HashMap::new();
                for (k, v) in [
                    (static_symbol!("void"), qbe::Type::Zero),
                    (static_symbol!("bool"), qbe::Type::Byte),
                    (static_symbol!("str"), qbe::Type::Long),
                    (static_symbol!("u8"), qbe::Type::UnsignedByte),
                    (static_symbol!("u16"), qbe::Type::UnsignedHalfword),
                    (static_symbol!("u32"), qbe::Type::Word),
                    (static_symbol!("u64"), qbe::Type::Long),
                    (static_symbol!("i8"), qbe::Type::Byte),
                    (static_symbol!("i16"), qbe::Type::Halfword),
                    (static_symbol!("i32"), qbe::Type::Word),
                    (static_symbol!("i64"), qbe::Type::Long),
                    (static_symbol!("f32"), qbe::Type::Single),
                    (static_symbol!("f64"), qbe::Type::Double),
                ] {
                    cache.insert(k, v);
                }
                cache
            },
            ..Default::default()
        }
    }

    fn emit_linkage(&self, linkage: &Linkage) -> qbe::Linkage {
        qbe::Linkage {
            exported: linkage.exported,
            ..Default::default()
        }
    }

    fn emit_aggregate(&mut self, aggregate: Aggregate) -> Arc<TypeDef> {
        let (name, td) = match aggregate {
            Aggregate::Struct { name, items } => (
                name,
                Arc::new(TypeDef::Regular {
                    ident: name.to_string(),
                    align: None,
                    items: items
                        .into_iter()
                        .map(|type_| (self.emit_type(&type_), 1))
                        .collect(),
                }),
            ),
            Aggregate::Union { name, variations } => (
                name,
                Arc::new(TypeDef::Union {
                    ident: name.to_string(),
                    align: None,
                    variations: variations
                        .into_iter()
                        .map(|items| {
                            items
                                .into_iter()
                                .map(|type_| (self.emit_type(&type_), 1))
                                .collect()
                        })
                        .collect(),
                }),
            ),
        };
        self.cache.insert(name, qbe::Type::aggregate(&td));
        td
    }

    fn emit_data_item(&mut self, item: DataItem) -> qbe::DataItem {
        match item {
            DataItem::Symbol(name, align) => qbe::DataItem::Symbol(name, align),
            DataItem::Str(content) => qbe::DataItem::Str(content),
            DataItem::Const(value) => qbe::DataItem::Const(value),
            DataItem::Zero(size) => qbe::DataItem::Zero(size),
        }
    }

    fn emit_type(&mut self, value: &Typed) -> qbe::Type {
        match value.as_ref() {
            type_::Type::Named { name, .. } => self
                .cache
                .get(name)
                .unwrap_or_else(|| panic!("expected type {name} exists"))
                .clone(),
            type_::Type::Fn { .. } | type_::Type::Ptr { .. } => qbe::Type::Long,
            type_::Type::Array { type_, size } => {
                let inner = self.emit_type(type_);
                let ident = format!("{}.array.{}", inner, size);
                let name = GlobalSymbol::from(&ident);
                if let Some(type_) = self.cache.get(&name) {
                    type_.clone()
                } else {
                    let td = Arc::new(TypeDef::Opaque {
                        ident,
                        align: inner.align(),
                        size: (*size) as u64 * inner.size(),
                    });
                    self.module.add_type(td.clone());
                    self.cache.insert(name, qbe::Type::aggregate(&td));
                    qbe::Type::aggregate(&td)
                }
            }
            type_::Type::Var { type_ } => match &*type_.read().unwrap() {
                type_::TypeVar::Unbound { .. } | type_::TypeVar::Generic { .. } => panic!(),
                type_::TypeVar::Link { type_ } => self.emit_type(&type_),
            },
        }
    }

    fn emit_value(&self, value: &Value) -> qbe::Value {
        match value.kind() {
            ValueKind::Global(symb) => qbe::Value::Global(symb.to_string()),
            ValueKind::Temporary(symb) | ValueKind::AutoLoad(symb) => {
                qbe::Value::Temporary(symb.to_string())
            }
            ValueKind::Const(value) => qbe::Value::Const(*value),
        }
    }
}

impl Emitter for QbeEmitter {
    type Instr = qbe::Instr;

    fn begin_func(
        &mut self,
        linkage: Linkage,
        name: GlobalSymbol,
        arguments: Vec<(GlobalSymbol, Typed)>,
        return_type: Option<Typed>,
    ) {
        self.function.linkage = self.emit_linkage(&linkage);
        self.function.name = name.to_string();
        self.function.arguments = arguments
            .into_iter()
            .map(|(name, type_)| {
                (
                    self.emit_type(&type_),
                    qbe::Value::Temporary(name.to_string()),
                )
            })
            .collect();
        self.function.return_ty = return_type.map(|type_| self.emit_type(&type_));
    }
    fn end_func(&mut self) {
        self.module
            .add_function(mem::replace(&mut self.function, function()));
    }

    fn begin_block(&mut self, label: GlobalSymbol) {
        self.block.label = label.to_string();
    }
    fn end_block(&mut self) {
        self.function
            .blocks
            .push(mem::replace(&mut self.block, block()));
    }

    fn layout_hint(&mut self, type_: &Typed) -> (u64, u64) {
        let type_ = self.emit_type(type_);
        (type_.align(), type_.size())
    }

    fn emit_type_def(&mut self, aggregate: Aggregate) {
        let aggregate = self.emit_aggregate(aggregate);
        self.module.add_type(aggregate);
    }

    fn emit_data_def(
        &mut self,
        linkage: Linkage,
        name: GlobalSymbol,
        items: Vec<(Typed, DataItem)>,
    ) {
        let linkage = self.emit_linkage(&linkage);
        let items = items
            .into_iter()
            .map(|(type_, item)| (self.emit_type(&type_), self.emit_data_item(item)))
            .collect();
        self.module
            .add_data(DataDef::new(linkage, name.to_string(), None, items));
    }

    fn emit_stmt_assign(&mut self, name: GlobalSymbol, type_: &Typed, init: Self::Instr) {
        let type_ = self.emit_type(type_);
        self.block
            .assign_instr(qbe::Value::Temporary(name.to_string()), type_, init);
    }
    fn emit_stmt_volatile(&mut self, instr: Self::Instr) {
        self.block.add_instr(instr)
    }

    fn emit_stmt_jmp(&mut self, label: GlobalSymbol) {
        let jmp = self.emit_instr_jmp(label);
        self.emit_stmt_volatile(jmp);
    }
    fn emit_stmt_store(&mut self, dest: &Value, src: &Value) {
        let store = self.emit_instr_store(dest, src);
        self.emit_stmt_volatile(store);
    }

    fn emit_instr_add(&mut self, left: Value, right: Value) -> Self::Instr {
        qbe::Instr::Add(self.emit_value(&left), self.emit_value(&right))
    }
    fn emit_instr_sub(&mut self, left: Value, right: Value) -> Self::Instr {
        qbe::Instr::Sub(self.emit_value(&left), self.emit_value(&right))
    }
    fn emit_instr_mul(&mut self, left: Value, right: Value) -> Self::Instr {
        qbe::Instr::Mul(self.emit_value(&left), self.emit_value(&right))
    }
    fn emit_instr_div(&mut self, left: Value, right: Value) -> Self::Instr {
        qbe::Instr::Div(self.emit_value(&left), self.emit_value(&right))
    }
    fn emit_instr_mod(&mut self, left: Value, right: Value) -> Self::Instr {
        qbe::Instr::Rem(self.emit_value(&left), self.emit_value(&right))
    }

    fn emit_instr_cmp(&mut self, cmp: Cmp, left: Value, right: Value) -> Self::Instr {
        let type_ = self.emit_type(left.type_());
        let cmp = match cmp {
            Cmp::Eq => qbe::Cmp::Eq,
            Cmp::Neq => qbe::Cmp::Ne,
            Cmp::Lt => qbe::Cmp::Slt,
            Cmp::Leq => qbe::Cmp::Sle,
            Cmp::Gt => qbe::Cmp::Sgt,
            Cmp::Geq => qbe::Cmp::Sge,
        };
        let left = self.emit_value(&left);
        let right = self.emit_value(&right);
        qbe::Instr::Cmp(type_, cmp, left, right)
    }

    fn emit_instr_and(&mut self, left: Value, right: Value) -> Self::Instr {
        qbe::Instr::And(self.emit_value(&left), self.emit_value(&right))
    }
    fn emit_instr_or(&mut self, left: Value, right: Value) -> Self::Instr {
        qbe::Instr::Or(self.emit_value(&left), self.emit_value(&right))
    }
    fn emit_instr_xor(&mut self, left: Value, right: Value) -> Self::Instr {
        qbe::Instr::Xor(self.emit_value(&left), self.emit_value(&right))
    }

    fn emit_instr_not(&mut self, right: Value) -> Self::Instr {
        qbe::Instr::Neg(self.emit_value(&right))
    }
    fn emit_instr_neg(&mut self, right: Value) -> Self::Instr {
        qbe::Instr::Neg(self.emit_value(&right))
    }
    fn emit_instr_copy(&mut self, value: Value) -> Self::Instr {
        qbe::Instr::Copy(self.emit_value(&value))
    }

    fn emit_instr_jmp(&mut self, label: GlobalSymbol) -> Self::Instr {
        qbe::Instr::Jmp(label.to_string())
    }
    fn emit_instr_jnz(
        &mut self,
        condition: Value,
        if_label: GlobalSymbol,
        else_label: GlobalSymbol,
    ) -> Self::Instr {
        qbe::Instr::Jnz(
            self.emit_value(&condition),
            if_label.to_string(),
            else_label.to_string(),
        )
    }
    fn emit_instr_call(&mut self, func: GlobalSymbol, args: Vec<Value>) -> Self::Instr {
        let args = args
            .into_iter()
            .map(|v| (self.emit_type(v.type_()), self.emit_value(&v)))
            .collect();
        qbe::Instr::Call(func.to_string(), args, None)
    }
    fn emit_instr_ret(&mut self, value: Option<Value>) -> Self::Instr {
        qbe::Instr::Ret(value.map(|sym| self.emit_value(&sym)))
    }

    fn emit_instr_alloca(&mut self, type_: Typed, align: Option<u64>) -> Self::Instr {
        let type_ = self.emit_type(&type_);
        let align = if let Some(align) = align {
            align
        } else {
            type_.align()
        };
        let size = type_.size();
        match align {
            4 => qbe::Instr::Alloc4(size.try_into().unwrap()),
            8 => qbe::Instr::Alloc8(size),
            16 => qbe::Instr::Alloc16(size.into()),
            _ => panic!("misaligned"),
        }
    }
    fn emit_instr_store(&mut self, dest: &Value, src: &Value) -> Self::Instr {
        let type_ = self.emit_type(src.type_());
        qbe::Instr::Store(type_, self.emit_value(dest), self.emit_value(src))
    }
    fn emit_instr_load(&mut self, type_: &Typed, src: &Value) -> Self::Instr {
        qbe::Instr::Load(self.emit_type(type_), self.emit_value(src))
    }
    fn emit_instr_phi(&mut self, values: Vec<(GlobalSymbol, Value)>) -> Self::Instr {
        let values = values
            .into_iter()
            .map(|(label, value)| (label.to_string(), self.emit_value(&value)))
            .collect();
        qbe::Instr::Phi(values)
    }
}

impl Unfold for QbeEmitter {
    type Unfolded = Module;

    fn unfold(self) -> Self::Unfolded {
        self.module
    }
}
