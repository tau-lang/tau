use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Display, Formatter, Result},
    rc::Rc,
};

use crate::ast::identifier::Identifier;

#[derive(Debug, PartialEq)]
pub struct Struct {
    pub name: Option<Identifier>,
    pub fields: TypeNames,
}

#[derive(Debug, PartialEq)]
pub struct Function {
    pub name: Option<Identifier>,
    pub parameters: Vec<Rc<TypeDef>>,
    pub return_type: Rc<TypeDef>,
}

#[derive(Debug, PartialEq)]
pub struct Number {
    pub size: u8,
    pub float: bool,
    pub signed: bool,
}

#[derive(Debug, PartialEq)]
pub enum TypeDef {
    Struct(Struct),
    Function(Function),
    Number(Number),
    Native(&'static str),
    Pointer(Rc<TypeDef>),
    Array(Rc<TypeDef>),
    Lazy(String),
    Unknown,
}

impl TypeDef {
    pub fn make_number(size: u8, float: bool, signed: bool) -> TypeDef {
        Self::Number(Number {
            size,
            float,
            signed,
        })
    }

    pub fn is_castable_to(&self, to: &Self) -> bool {
        match self {
            Self::Number(number) => {
                let (this_size, this_float) = (number.size, number.float);
                if let Self::Number(number) = to {
                    if this_float {
                        return number.float && this_size <= number.size;
                    } else {
                        return this_size <= number.size;
                    }
                } else {
                    return false;
                }
            }
            Self::Function(function) => {
                let (this_para, this_return) = (&function.parameters, &function.return_type);
                if let Self::Function(function) = to {
                    for (par_from, par_to) in this_para.iter().zip(&function.parameters) {
                        if !par_from.is_castable_to(&par_to) {
                            return false;
                        }
                    }
                    return this_return.is_castable_to(&function.return_type);
                } else {
                    return false;
                }
            }
            _ => self == to,
        }
    }

    pub fn is_integer(&self) -> bool {
        if let TypeDef::Number(number) = self {
            return !number.float;
        }
        false
    }

    pub fn is_number(&self) -> bool {
        if let TypeDef::Number(_) = self {
            return true;
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
            Self::Struct(fields) => write!(formatter, "{:#?}", fields),
            Self::Function(function) => write!(
                formatter,
                "{:?} -> {}",
                function.parameters, function.return_type
            ),
            Self::Pointer(pointer_type) => write!(formatter, "*{}", pointer_type),
            Self::Array(array_type) => write!(formatter, "{}[]", array_type),
            Self::Number(number) => write!(
                formatter,
                "{}{}",
                if number.float {
                    'f'
                } else if number.signed {
                    'i'
                } else {
                    'u'
                },
                number.size
            ),
            Self::Native(name) => formatter.write_str(name),
            Self::Lazy(name) => formatter.write_str(name),
            Self::Unknown => formatter.write_str("Unknown"),
        }
    }
}

pub type TypeNames = HashMap<String, Rc<TypeDef>>;

pub type TypeCell = RefCell<Rc<TypeDef>>;
