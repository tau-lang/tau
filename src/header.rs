use std::{collections::HashMap, fs, path::PathBuf, rc::Rc};

use crate::{
    ast::{
        declaration::{Decl, DeclVisitor},
        expression::Expr,
        identifier::Identifier,
        statement::Stmt,
    },
    lexer::Lexer,
    parser::Parser,
    typing::{TypeCell, TypeDef, TypeNames},
};

pub struct Header {
    types: TypeNames,
    fields: TypeNames,
}

impl Header {
    pub fn new() -> Header {
        Header {
            types: HashMap::from([
                (
                    "u8".to_string(),
                    Rc::new(TypeDef::make_number("u8", 1, false, false)),
                ),
                (
                    "u16".to_string(),
                    Rc::new(TypeDef::make_number("u16", 2, false, false)),
                ),
                (
                    "u32".to_string(),
                    Rc::new(TypeDef::make_number("u32", 4, false, false)),
                ),
                (
                    "u64".to_string(),
                    Rc::new(TypeDef::make_number("u64", 8, false, false)),
                ),
                (
                    "i8".to_string(),
                    Rc::new(TypeDef::make_number("i8", 1, false, true)),
                ),
                (
                    "i16".to_string(),
                    Rc::new(TypeDef::make_number("i16", 2, false, true)),
                ),
                (
                    "i32".to_string(),
                    Rc::new(TypeDef::make_number("i32", 4, false, true)),
                ),
                (
                    "i64".to_string(),
                    Rc::new(TypeDef::make_number("i64", 8, false, true)),
                ),
                (
                    "f32".to_string(),
                    Rc::new(TypeDef::make_number("f32", 4, true, true)),
                ),
                (
                    "f64".to_string(),
                    Rc::new(TypeDef::make_number("f64", 8, true, true)),
                ),
                ("bool".to_string(), Rc::new(TypeDef::Native("bool"))),
                ("char".to_string(), Rc::new(TypeDef::Native("char"))),
                ("str".to_string(), Rc::new(TypeDef::Native("str"))),
                ("void".to_string(), Rc::new(TypeDef::Native("void"))),
            ]),
            fields: HashMap::new(),
        }
    }

    pub fn headers(mut self, declarations: &[Decl]) -> Header {
        for declaration in declarations {
            self.visit_decl(declaration);
        }
        self
    }

    pub fn analysed(self) -> (TypeNames, TypeNames) {
        (self.types, self.fields)
    }

    fn make_function_type(
        &self,
        return_type: &TypeCell,
        params: &[(Identifier, TypeCell)],
    ) -> Rc<TypeDef> {
        let mut parameters = Vec::new();
        for (_, param_type) in params {
            parameters.push(param_type.borrow().clone());
        }

        Rc::new(TypeDef::Function {
            parameters,
            return_type: return_type.borrow().clone(),
        })
    }
}

impl<'a> DeclVisitor<'a> for Header {
    type Output = ();

    fn visit_import(&mut self, path: &[Identifier]) {
        let mut module_name = "";
        let filename = {
            let mut filename = PathBuf::new();
            for name in path {
                filename.push(name.get_name());
                module_name = name.get_name();
            }
            filename.set_extension("tau");
            filename
        };
        let content = fs::read_to_string(&filename).expect("Expected to open file");
        let lexer = Lexer::new(content.chars(), Rc::new(filename));
        let parser = Parser::new(lexer.scan().unwrap());
        // FIX:
        let ast = parser.parse().unwrap();
        let header = Header::new().headers(&ast);
        let (types, fields) = header.analysed();
        self.fields.insert(
            module_name.to_string(),
            Rc::new(TypeDef::Module { types, fields }),
        );
    }

    fn visit_struct(
        &mut self,
        name: &'a Identifier,
        fields: &'a [(Identifier, TypeCell)],
        methods: &'a [Rc<Decl>],
    ) {
        let struct_name = name.get_name().to_string();

        let mut members = HashMap::new();
        for (field_name, field_type) in fields {
            members.insert(
                field_name.get_name().to_string(),
                field_type.borrow().clone(),
            );
        }
        for decl in methods {
            if let Decl::Function {
                name,
                return_type,
                params,
                ..
            } = decl.as_ref()
            {
                members.insert(
                    name.get_name().to_string(),
                    self.make_function_type(return_type, params),
                );
            }
        }

        self.types.insert(
            struct_name,
            Rc::new(TypeDef::Struct {
                name: name.clone(),
                members,
            }),
        );
    }

    fn visit_function(
        &mut self,
        name: &'a Identifier,
        return_type: &'a TypeCell,
        params: &'a [(Identifier, TypeCell)],
        _: &[Stmt],
        _: bool,
        _: bool,
    ) {
        self.fields.insert(
            name.get_name().to_string(),
            self.make_function_type(return_type, params),
        );
    }

    fn visit_const(&mut self, name: &'a Identifier, var_type: &'a TypeCell, _: &Expr) {
        self.fields
            .insert(name.get_name().to_string(), var_type.borrow().clone());
    }
}
