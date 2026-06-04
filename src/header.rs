use std::{collections::HashMap, fs, path::PathBuf, rc::Rc};

use crate::{
    ast::{
        declaration::{Decl, DeclVisitor, Function, Structure},
        expression::Expr,
        identifier::Identifier,
    },
    lexer::Lexer,
    parser::Parser,
    typing::{self, TypeCell, TypeDef, TypeNames, TypeTree},
};

pub struct Header {
    types: TypeTree,
    scope: TypeNames,
}

impl Header {
    pub fn new() -> Header {
        Header {
            types: TypeTree::new(),
            scope: HashMap::new(),
        }
    }

    pub fn headers(mut self, declarations: &[Decl]) -> Header {
        for declaration in declarations {
            self.visit_decl(declaration);
        }
        self
    }

    pub fn analysed(self) -> (TypeTree, TypeNames) {
        (self.types, self.scope)
    }
}

impl<'a> DeclVisitor<'a> for Header {
    type Output = ();

    fn visit_import(&mut self, path: &[Identifier]) {
        let filename = {
            let mut filename = PathBuf::new();
            for name in path {
                filename.push(name.name());
            }
            filename.set_extension("tau");
            filename
        };
        let content = fs::read_to_string(&filename).expect("Expected to open file");
        let lexer = Lexer::new(content.chars(), Rc::new(filename));
        let parser = Parser::new(lexer.scan().unwrap());

        let ast = parser.parse().unwrap();
        let header = Header::new().headers(&ast);
        let (types, scope) = header.analysed();
        // TODO: error handling
        self.types
            .insert_tree(path.last().unwrap().to_string(), types);
        self.scope.extend(scope);
    }

    fn visit_struct(&mut self, structure: &'a Structure) {
        let struct_name = structure.name.name().to_string();

        let mut fields = HashMap::new();
        for (field_name, field_type) in &structure.fields {
            fields.insert(field_name.name().to_string(), field_type.borrow().clone());
        }
        self.types.insert_type(
            struct_name,
            Rc::new(TypeDef::Struct(typing::Struct {
                name: vec![structure.name.clone()],
                fields,
            })),
        );
    }

    fn visit_function(&mut self, func: &'a Function) {
        let mut parameters = Vec::new();
        for (_, param_type) in &func.params {
            parameters.push(param_type.borrow().clone());
        }
        self.scope.insert(
            func.name.name().to_string(),
            Rc::new(TypeDef::Function(typing::Function {
                name: vec![
                    // TODO: add full module path
                    func.name.clone(),
                ],
                parameters,
                return_type: func.return_type.borrow().clone(),
            })),
        );
    }

    fn visit_const(&mut self, name: &'a Identifier, var_type: &'a TypeCell, _: &Expr) {
        self.scope
            .insert(name.name().to_string(), var_type.borrow().clone());
    }
}
