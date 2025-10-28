use crate::ast::*;
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Display, Formatter, Result},
    rc::Rc,
};

#[derive(Debug, PartialEq)]
pub enum TypeDef {
    Module {
        types: TypeNames,
        fields: TypeNames,
    },
    Struct {
        name: Identifier,
        members: HashMap<String, Rc<TypeDef>>,
    },
    Function {
        parameters: Vec<Rc<TypeDef>>,
        return_type: Rc<TypeDef>,
    },
    Number {
        name: &'static str,
        size: u8,
        float: bool,
        signed: bool,
    },
    Native(&'static str),
    Array(Rc<TypeDef>),
    Lazy(String),
    Unknown,
}

impl TypeDef {
    pub fn make_number(name: &'static str, size: u8, float: bool, signed: bool) -> TypeDef {
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

impl Display for TypeDef {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        match self {
            Self::Module {
                types: _,
                fields: _,
            } => formatter.write_str("<module>"),
            Self::Struct { name, members: _ } => formatter.write_str(name.get_name()),
            Self::Function {
                parameters,
                return_type,
            } => write!(formatter, "{:?} -> {}", parameters, return_type),
            Self::Array(array_type) => {
                write!(formatter, "{:?}[]", array_type)
            }
            Self::Number {
                name,
                size: _,
                float: _,
                signed: _,
            }
            | Self::Native(name) => formatter.write_str(name),
            Self::Lazy(name) => formatter.write_str(&name),
            Self::Unknown => formatter.write_str("Unknown"),
        }
    }
}

pub type TypeNames = HashMap<String, Rc<TypeDef>>;

pub type TypeCell = RefCell<Rc<TypeDef>>;
