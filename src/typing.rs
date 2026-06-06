use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Display, Formatter, Result},
    rc::Rc,
};

use crate::{
    ast::identifier::Identifier,
    error::{self, Diagnostic, Error},
};

#[derive(Debug, PartialEq)]
pub struct Struct {
    pub name: TypePath,
    pub fields: TypeNames,
}

#[derive(Debug, PartialEq)]
pub struct Function {
    pub name: TypePath,
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
    RawPointer(Rc<TypeDef>),
    Array(Rc<TypeDef>),
    Path(TypePath),
    Any,
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
            Self::Any => true,
            Self::RawPointer(ptr) => {
                if let Self::RawPointer(other) = to {
                    ptr.is_castable_to(other)
                } else {
                    false
                }
            }
            _ => {
                if let Self::Any = to {
                    true
                } else {
                    self == to
                }
            }
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
            Self::Struct(structure) => {
                formatter.write_str(structure.name[0].name())?;
                for item in &structure.name[1..] {
                    formatter.write_str("::")?;
                    formatter.write_str(item.name())?;
                }
                Ok(())
            }
            Self::Function(function) => write!(
                formatter,
                "{:?} -> {}",
                function.parameters, function.return_type
            ),
            Self::RawPointer(pointer_type) => write!(formatter, "*{pointer_type}"),
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
            Self::Path(path) => {
                formatter.write_str(path[0].name())?;
                for item in &path[1..] {
                    formatter.write_str("::")?;
                    formatter.write_str(item.name())?;
                }
                Ok(())
            }
            Self::Any => formatter.write_str("Any"),
            Self::Unknown => formatter.write_str("Unknown"),
        }
    }
}

pub type TypePath = Vec<Identifier>;

pub type TypeNames = HashMap<String, Rc<TypeDef>>;

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
            Rc::new(TypeDef::make_number(size, float, signed)),
        )
    }};
}

pub struct TypeTree {
    childs: HashMap<String, TypeTree>,
    value: Option<Rc<TypeDef>>,
}

impl TypeTree {
    pub fn new() -> Self {
        let childs = HashMap::from(
            [
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
                ("any".to_string(), Rc::new(TypeDef::Any)),
                ("bool".to_string(), Rc::new(TypeDef::Native("bool"))),
                ("char".to_string(), Rc::new(TypeDef::Native("char"))),
                ("str".to_string(), Rc::new(TypeDef::Native("str"))),
                ("void".to_string(), Rc::new(TypeDef::Native("void"))),
            ]
            .map(|(key, value)| {
                (
                    key,
                    TypeTree {
                        childs: HashMap::new(),
                        value: Some(value),
                    },
                )
            }),
        );
        TypeTree {
            childs,
            value: None,
        }
    }

    pub fn lookup_name(&self, name: &str) -> Rc<TypeDef> {
        // TODO: Error handling! To many clones and unwraps!
        self.childs
            .get(name)
            .expect("expect name exists")
            .value
            .clone()
            .unwrap()
            .clone()
    }

    pub fn lookup_type(&self, type_def: &Rc<TypeDef>) -> error::Result<Rc<TypeDef>> {
        match type_def.as_ref() {
            TypeDef::Path(path) => self.lookup_path(path),
            TypeDef::RawPointer(ptr) => {
                if let TypeDef::Path(path) = ptr.as_ref() {
                    Ok(Rc::new(TypeDef::RawPointer(self.lookup_path(path)?)))
                } else {
                    Ok(type_def.clone())
                }
            }
            _ => Ok(type_def.clone()),
        }
    }

    pub fn lookup_path(&self, path: &TypePath) -> error::Result<Rc<TypeDef>> {
        let mut tree = self;
        for identidier in path {
            if let Some(child) = tree.childs.get(identidier.name()) {
                tree = child;
            } else {
                return Err(Error::new(vec![Diagnostic::new(
                    "could not find path".to_string(),
                    identidier.source(),
                )]));
            }
        }
        if let Some(value) = &tree.value {
            Ok(value.clone())
        } else {
            // TODO: add diagnostics that the type could not be found.
            Err(Error::new(vec![]))
        }
    }

    pub fn insert_type(&mut self, name: String, type_def: Rc<TypeDef>) -> Option<TypeTree> {
        self.insert_tree(
            name,
            TypeTree {
                childs: HashMap::new(),
                value: Some(type_def),
            },
        )
    }

    pub fn insert_tree(&mut self, name: String, type_tree: TypeTree) -> Option<TypeTree> {
        self.childs.insert(name, type_tree)
    }
}

pub type TypeCell = RefCell<Rc<TypeDef>>;
