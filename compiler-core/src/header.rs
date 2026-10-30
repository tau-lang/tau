use crate::{
    ast::{
        UntypedDecl,
        declaration::{DeclFunc, DeclStruct, DeclVisitor},
        expression::Expr,
        identifier::Identifier,
    },
    context::Context,
    error::Result,
    lexer::Source,
    process_file,
    type_::{
        Type,
        environment::{Environment, ModuleInterface, StructInterface},
        prelude::Typed,
    },
};
use std::{fs, path::PathBuf, sync::Arc};
use symbol_table::{GlobalSymbol, static_symbol};

pub struct Header<'ctx> {
    context: &'ctx mut Context,
    module: ModuleInterface,
}

impl<'ctx> Header<'ctx> {
    pub fn new(context: &'ctx mut Context) -> Self {
        Self {
            context,
            // TODO: add real module name
            module: ModuleInterface::new(static_symbol!("")),
        }
    }

    pub fn env(mut self, declarations: &[UntypedDecl]) -> Result<Environment> {
        for declaration in declarations {
            self.visit_decl(declaration)?;
        }
        let module = Arc::new(self.module);
        self.context.cache_module(module.clone());
        Ok(Environment::new(self.context.ids().clone(), module))
    }
}

impl<'a, 'ctx> DeclVisitor<'a> for Header<'ctx> {
    type Type = ();
    type Output = Result<()>;

    fn visit_import(&mut self, path: &[Identifier], _: Source) -> Self::Output {
        let filename = {
            let mut filename = PathBuf::new();
            for name in path {
                filename.push(name.name());
            }
            filename.set_extension("tau");
            filename
        };
        let content = fs::read_to_string(&filename).expect("Expected to open file");
        let display = filename.display();
        // TODO: remove clone!
        let file_id = self.context.store(display.to_string(), content.clone());
        let (_ast, _env) = process_file(self.context, file_id, &content)?;

        // TODO: insert module
        Ok(())
    }

    fn visit_struct(&mut self, structure: &'a DeclStruct, source: Source) -> Self::Output {
        let mut struct_ = StructInterface::new(structure.name.symbol(), source);
        for (field_name, field_type) in &structure.fields {
            struct_.declare_associaten(field_name, field_type.clone())?;
        }
        self.module.declare_type(
            &structure.name,
            Arc::new(Type::Named {
                name: GlobalSymbol::from(structure.name.name()),
                // TODO: add generics
                arguments: vec![],
            }),
        )?;
        self.module.declare_struct(struct_)
    }

    fn visit_function(&mut self, func: &'a DeclFunc<Self::Type>, _: Source) -> Self::Output {
        let mut arguments = Vec::new();
        for (_, type_) in &func.params {
            arguments.push(type_.clone());
        }
        self.module.declare_var(
            &func.name,
            Arc::new(Type::Fn {
                arguments,
                return_: func.return_type.clone(),
            }),
        )
    }

    fn visit_const(
        &mut self,
        name: &'a Identifier,
        type_: &'a Typed,
        _: &Expr<Self::Type>,
        _: Source,
    ) -> Self::Output {
        self.module.declare_var(name, type_.clone())
    }
}
