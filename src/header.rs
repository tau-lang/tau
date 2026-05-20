use std::{collections::HashMap, fs, path::PathBuf, rc::Rc};

use crate::{
    ast::{
        declaration::{Decl, DeclVisitor, Function, Structure},
        expression::Expr,
        identifier::Identifier,
    },
    lexer::Lexer,
    parser::Parser,
    typing::{TypeCell, TypeDef, TypeNames},
};

/// This macro takes the name of a number type and returns a tuple `(String,
/// Rc<TypeDef>)`, where the first entry is the name of the type and the
/// second the full type definition. The type definition itself contains if
/// the type is signed, if it is a float and the size of a number of the
/// type.
#[macro_export]
macro_rules! number {
    ( $name:expr ) => {{
        let (float, signed) = match $name.chars().nth(0).expect("first char exists") {
            'u' => (false, false),
            'i' => (false, true),
            'f' => (true, true),
            _ => panic!("number should start with u,i or f"),
        };
        let size = match &$name[1..] {
            "8" => 8,
            "16" => 16,
            "32" => 32,
            "64" => 64,
            _ => panic!("number should have size 8, 16, 32 or 64"),
        };
        (
            $name.to_string(),
            Rc::new(TypeDef::make_number($name, size, float, signed)),
        )
    }};
}

pub struct Header {
    types: TypeNames,
    fields: TypeNames,
}

impl Header {
    pub fn new() -> Header {
        Header {
            types: HashMap::from([
                number!("u8"),
                number!("u16"),
                number!("u32"),
                number!("u64"),
                number!("i8"),
                number!("i16"),
                number!("i32"),
                number!("i64"),
                number!("f32"),
                number!("f64"),
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
                filename.push(name.name());
                module_name = name.name();
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

    fn visit_struct(&mut self, structure: &'a Structure) {
        let struct_name = structure.name.name().to_string();

        let mut members = HashMap::new();
        for (field_name, field_type) in &structure.fields {
            members.insert(field_name.name().to_string(), field_type.borrow().clone());
        }
        for decl in &structure.methods {
            if let Decl::Function(func) = decl.as_ref() {
                members.insert(
                    structure.name.name().to_string(),
                    self.make_function_type(&func.return_type, &func.params),
                );
            }
        }

        self.types.insert(
            struct_name,
            Rc::new(TypeDef::Struct {
                name: structure.name.clone(),
                members,
            }),
        );
    }

    fn visit_function(&mut self, func: &'a Function) {
        self.fields.insert(
            func.name.name().to_string(),
            self.make_function_type(&func.return_type, &func.params),
        );
    }

    fn visit_const(&mut self, name: &'a Identifier, var_type: &'a TypeCell, _: &Expr) {
        self.fields
            .insert(name.name().to_string(), var_type.borrow().clone());
    }
}
