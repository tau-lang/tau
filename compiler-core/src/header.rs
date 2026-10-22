use crate::{
    ast::{
        Typed, UntypedDecl,
        declaration::{DeclFunc, DeclStruct, DeclVisitor},
        expression::Expr,
        identifier::Identifier,
    },
    error::{FileDb, Result},
    lexer::Source,
    process_file,
    type_::Type,
};
use codespan_reporting::term::{
    self,
    termcolor::{ColorChoice, StandardStream},
};
use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};
use symbol_table::GlobalSymbol;

pub type TypeMap = HashMap<GlobalSymbol, Typed>;

pub struct Header {
    types: TypeMap,
    scope: TypeMap,
}

impl Header {
    pub fn new() -> Header {
        Header {
            types: TypeMap::new(),
            scope: TypeMap::new(),
        }
    }

    pub fn headers(mut self, declarations: &[UntypedDecl]) -> Result<Header> {
        for declaration in declarations {
            self.visit_decl(declaration)?;
        }
        Ok(self)
    }

    pub fn analysed(self) -> (TypeMap, TypeMap) {
        (self.types, self.scope)
    }
}

impl<'a> DeclVisitor<'a> for Header {
    type Type = ();
    type Output = Result<()>;

    fn visit_import(&mut self, path: &[Identifier], _: &Source) -> Self::Output {
        let filename = {
            let mut filename = PathBuf::new();
            for name in path {
                filename.push(name.name());
            }
            filename.set_extension("tau");
            filename
        };
        let content = fs::read_to_string(&filename).expect("Expected to open file");
        let mut files = FileDb::new();
        let display = filename.display();
        let file_id = files.add(&display, &content);
        match process_file(file_id, &content) {
            Ok((_, _, scope)) => {
                // TODO: insert module
                /*self.types.insert(
                    path.last().expect("import path not empty").to_string(),
                    types,
                );*/
                self.scope.extend(scope);
            }
            Err(error) => {
                for diagnostic in error.diagnostics() {
                    let writer = StandardStream::stderr(ColorChoice::Always);
                    let config = codespan_reporting::term::Config::default();

                    let _ =
                        term::emit_to_io_write(&mut writer.lock(), &config, &files, &diagnostic);
                }
            }
        }
        Ok(())
    }

    fn visit_struct(&mut self, structure: &'a DeclStruct, _: &Source) -> Self::Output {
        let struct_name = GlobalSymbol::from(structure.name.name());

        let mut fields = Vec::new();
        for (field_name, field_type) in &structure.fields {
            fields.push((field_name.name().to_string(), field_type.clone()));
        }
        self.types.insert(
            struct_name,
            Arc::new(Type::Named {
                name: GlobalSymbol::from(structure.name.name()),
                arguments: vec![],
            }),
        );
        Ok(())
    }

    fn visit_function(&mut self, func: &'a DeclFunc<Self::Type>, _: &Source) -> Self::Output {
        let mut arguments = Vec::new();
        for (_, type_) in &func.params {
            arguments.push(type_.clone());
        }
        self.types.insert(
            GlobalSymbol::from(func.name.name()),
            Arc::new(Type::Fn {
                arguments,
                return_: func.return_type.clone(),
            }),
        );
        Ok(())
    }

    fn visit_const(
        &mut self,
        name: &'a Identifier,
        type_: &'a Typed,
        _: &Expr<Self::Type>,
        _: &Source,
    ) -> Self::Output {
        self.scope
            .insert(GlobalSymbol::from(name.name()), type_.clone());
        Ok(())
    }
}
