use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result};
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
        parameters: Vec<Rc<TypeDef<'a>>>,
        return_type: Rc<TypeDef<'a>>,
    },
    Array(Rc<TypeDef<'a>>),
    Number {
        name: &'static str,
        size: u8,
        float: bool,
        signed: bool,
    },
    Native(&'static str),
    Lazy(&'a str),
}

impl<'a> TypeDef<'a> {
    fn make_number(name: &'static str, size: u8, float: bool, signed: bool) -> TypeDef<'a> {
        Self::Number {
            name,
            size,
            float,
            signed,
        }
    }

    pub fn is_castable_to(&self, to: &Self) -> bool {
        match self {
            Self::Number {
                name: _,
                size,
                float,
                signed: _,
            } => {
                let (this_size, this_float) = (size, float);
                if let Self::Number {
                    name: _,
                    size,
                    float,
                    signed: _,
                } = to
                {
                    if *this_float {
                        return *float && this_size <= size;
                    } else {
                        return this_size <= size;
                    }
                } else {
                    return false;
                }
            }
            Self::Function {
                parameters,
                return_type,
            } => {
                let (this_para, this_return) = (parameters, return_type);
                if let Self::Function {
                    parameters,
                    return_type,
                } = to
                {
                    for (par_from, par_to) in this_para.iter().zip(parameters) {
                        if !par_from.is_castable_to(par_to) {
                            return false;
                        }
                    }
                    return this_return.is_castable_to(return_type);
                } else {
                    return false;
                }
            }
            Self::Struct { name, members: _ } => {
                let this_name = name;
                if let TypeDef::Struct { name, members: _ } = to {
                    return this_name == name;
                } else {
                    return false;
                }
            }
            Self::Native(this_name) => {
                if let Self::Native(name) = to {
                    return this_name == name;
                } else {
                    return false;
                }
            }
            _ => {
                panic!(
                    "tried to typecheck the lazy type '{}' that was not dereferenced yet",
                    self
                )
            }
        }
    }

    pub fn is_integer(&self) -> bool {
        if let TypeDef::Number {
            name: _,
            size: _,
            float,
            signed: _,
        } = *self
        {
            return !float;
        }
        false
    }

    pub fn is_number(&self) -> bool {
        if let TypeDef::Number {
            name,
            size: _,
            float: _,
            signed: _,
        } = *self
        {
            return matches!(
                name,
                "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "f32" | "f64"
            );
        }
        false
    }

    pub fn is_bool(&self) -> bool {
        if let TypeDef::Native(name) = *self {
            name == "bool"
        } else {
            false
        }
    }
}

impl Display for TypeDef<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        match self {
            Self::Struct { name, members: _ } => formatter.write_str(name),
            Self::Function {
                parameters,
                return_type,
            } => write!(formatter, "{:?} -> {}", parameters, return_type),
            Self::Array(array_type) => write!(formatter, "[{:?}]", array_type),
            Self::Number {
                name,
                size: _,
                float: _,
                signed: _,
            } => formatter.write_str(name),
            Self::Native(name) => formatter.write_str(name),
            Self::Lazy(name) => formatter.write_str(name),
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
                ("u8", Rc::new(TypeDef::make_number("u8", 1, false, false))),
                ("u16", Rc::new(TypeDef::make_number("u16", 2, false, false))),
                ("u32", Rc::new(TypeDef::make_number("u32", 4, false, false))),
                ("u64", Rc::new(TypeDef::make_number("u64", 8, false, false))),
                ("i8", Rc::new(TypeDef::make_number("i8", 1, false, true))),
                ("i16", Rc::new(TypeDef::make_number("i16", 2, false, true))),
                ("i32", Rc::new(TypeDef::make_number("i32", 4, false, true))),
                ("i64", Rc::new(TypeDef::make_number("i64", 8, false, true))),
                ("f32", Rc::new(TypeDef::make_number("f32", 4, true, true))),
                ("f64", Rc::new(TypeDef::make_number("f64", 8, true, true))),
                ("bool", Rc::new(TypeDef::Native("bool"))),
                ("char", Rc::new(TypeDef::Native("char"))),
                ("str", Rc::new(TypeDef::Native("str"))),
                ("void", Rc::new(TypeDef::Native("void"))),
            ]),
            fields: HashMap::new(),
        }
    }

    pub fn headers(mut self, declarations: &'a [Decl]) -> Header<'a> {
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

    fn make_function_type(
        &self,
        return_type: &'a Option<Token>,
        params: &'a [(Token, Token)],
    ) -> Rc<TypeDef<'a>> {
        let mut parameters = Vec::new();
        for (_, param_type) in params {
            parameters.push(self.get_type(param_type));
        }

        let return_type = if let Some(type_name) = return_type {
            self.get_type(type_name)
        } else {
            self.types
                .get("void")
                .expect("expected void type exists")
                .clone()
        };

        Rc::new(TypeDef::Function {
            parameters,
            return_type,
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
        fields: &'a [(Token, Token)],
        methods: &'a [Rc<Decl>],
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
                    self.make_function_type(return_type, params),
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
        return_type: &'a Option<Token>,
        params: &'a [(Token, Token)],
        _: &[Stmt],
    ) {
        self.fields.insert(
            name.identifier(),
            self.make_function_type(return_type, params),
        );
    }

    fn visit_const(&mut self, name: &'a Token, var_type: &'a Token, _: &Expr) {
        self.fields
            .insert(name.identifier(), self.get_type(var_type));
    }
}
