use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::{Decl, DeclVisitor, Expr, Stmt};
use crate::lexer::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeDef<'a> {
    Struct {
        name: &'a str,
        members: HashMap<&'a str, Rc<TypeDef<'a>>>,
    },
    Function {
        name: &'a str,
        parameters: HashMap<&'a str, Rc<TypeDef<'a>>>,
        return_type: Rc<TypeDef<'a>>,
    },
    Native(&'static str),
    Lazy(&'a str),
}

impl TypeDef<'_> {
    pub fn is_integer(&self) -> bool {
        if let TypeDef::Native(type_name) = *self {
            match type_name {
                "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" => true,
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn is_number(&self) -> bool {
        if let TypeDef::Native(type_name) = *self {
            match type_name {
                "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "f32" | "f64" => true,
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn is_bool(&self) -> bool {
        if let TypeDef::Native(type_name) = *self {
            type_name == "bool"
        } else {
            false
        }
    }
}

pub struct Header<'a> {
    types: HashMap<&'a str, Rc<TypeDef<'a>>>,
    fields: HashMap<&'a str, Rc<TypeDef<'a>>>,
}

impl<'a> Header<'a> {
    pub fn new() -> Header<'a> {
        Header {
            types: HashMap::from([
                ("u8", Rc::new(TypeDef::Native("u8"))),
                ("u16", Rc::new(TypeDef::Native("u16"))),
                ("u32", Rc::new(TypeDef::Native("u32"))),
                ("u64", Rc::new(TypeDef::Native("u64"))),
                ("i8", Rc::new(TypeDef::Native("i8"))),
                ("i16", Rc::new(TypeDef::Native("i16"))),
                ("i32", Rc::new(TypeDef::Native("i32"))),
                ("i64", Rc::new(TypeDef::Native("i64"))),
                ("f32", Rc::new(TypeDef::Native("f32"))),
                ("f64", Rc::new(TypeDef::Native("f64"))),
                ("bool", Rc::new(TypeDef::Native("bool"))),
                ("char", Rc::new(TypeDef::Native("char"))),
                ("str", Rc::new(TypeDef::Native("str"))),
                ("void", Rc::new(TypeDef::Native("void"))),
            ]),
            fields: HashMap::new(),
        }
    }

    pub fn headers(mut self, declarations: &'a Vec<Decl>) -> Header<'a> {
        for declaration in declarations {
            self.visit_decl(declaration);
        }
        self
    }

    pub fn analysed(
        self,
    ) -> (
        HashMap<&'a str, Rc<TypeDef<'a>>>,
        HashMap<&'a str, Rc<TypeDef<'a>>>,
    ) {
        (self.types, self.fields)
    }

    fn make_function(
        &self,
        name: &'a Token,
        return_type: &'a Token,
        params: &'a Vec<(Token, Token)>,
    ) -> Rc<TypeDef<'a>> {
        let mut parameters = HashMap::new();
        for (param_name, param_type) in params {
            parameters.insert(param_name.identifier(), self.get_type(param_type));
        }
        Rc::new(TypeDef::Function {
            name: name.identifier(),
            parameters,
            return_type: self.get_type(return_type),
        })
    }

    fn get_type(&self, name: &'a Token) -> Rc<TypeDef<'a>> {
        let ref_name = name.identifier();
        if let Some(ref_type) = self.types.get(ref_name) {
            ref_type.clone()
        } else {
            Rc::new(TypeDef::Lazy(ref_name))
        }
    }
}

impl<'a> DeclVisitor<'a, ()> for Header<'a> {
    fn visit_import(&mut self, _: &'a Token) {
        // TODO: open import file and add imported fields and types here
    }

    fn visit_struct(
        &mut self,
        name: &'a Token,
        fields: &'a Vec<(Token, Token)>,
        methods: &'a Vec<Rc<Decl>>,
    ) {
        let struct_name = name.identifier();

        let mut members = HashMap::new();
        for (field_name, field_type) in fields {
            members.insert(field_name.identifier(), self.get_type(field_type));
        }
        for decl in methods {
            if let Decl::Function {
                name,
                return_type,
                params,
                ..
            } = &**decl
            {
                members.insert(
                    name.identifier(),
                    self.make_function(name, return_type, params),
                );
            }
        }

        self.types.insert(
            struct_name,
            Rc::new(TypeDef::Struct {
                name: struct_name,
                members,
            }),
        );
    }

    fn visit_function(
        &mut self,
        name: &'a Token,
        return_type: &'a Token,
        params: &'a Vec<(Token, Token)>,
        _: &Stmt,
    ) {
        self.fields.insert(
            name.identifier(),
            self.make_function(name, return_type, params),
        );
    }

    fn visit_const(&mut self, name: &'a Token, var_type: &'a Token, _: &Expr) {
        self.fields
            .insert(name.identifier(), self.get_type(var_type));
    }
}
