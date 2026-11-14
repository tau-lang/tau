//! Type system for Tau. Tau is statically and strongly typed with (currently
//! minimum) type inference.
//!
//! The compiler currently detects the following types:
//! - Named types like structs or bool
//! - Function types
//! - Array types
//! - Pointer types
//!
//! The following types are planned but not yet implemented:
//! - Tuple types with an empty tuple replacing the void type
//! - Never type for indicating that an expression or statement never returns
//! - Error types (internal only) for marking nodes in the ast that have an
//!   error. This allows for better error reporting.

use crate::error::FileId;
use codespan_reporting::diagnostic::Diagnostic;
use std::{
    fmt,
    ops::Deref,
    sync::{Arc, RwLock},
};
use symbol_table::{GlobalSymbol, static_symbol};

pub mod declaration;
pub mod environment;
pub mod expression;
pub mod prelude;
pub mod statement;
pub mod type_var;
pub mod uid;

pub use crate::type_::prelude::*;
pub use type_var::{TypeVar, collapse_links};

#[derive(Clone, Debug)]
pub enum Type {
    /// A named type such as i32, f64 or a custom type such as Vec.
    /// The type can take other types as generic arguments
    Named {
        // TODO: bring them back when we have a better implementation for
        // imports package: GlobalSymbol,
        // module: GlobalSymbol,
        name: GlobalSymbol,
        /// List of all generic types, can be empty.
        arguments: Vec<Arc<Type>>,
    },

    /// Type of a function.
    Fn {
        arguments: Vec<Arc<Type>>,
        return_: Arc<Type>,
    },

    Var {
        type_: Arc<RwLock<TypeVar>>,
    },

    Array {
        type_: Arc<Type>,
        size: usize,
    },

    /// Reference or pointer to another type
    Ptr {
        /// The referenced type
        type_: Arc<Type>,
        /// Raw pointers (*) are unchecked, references are checked (&)
        unchecked: bool,
    },
}

impl Type {
    #[must_use]
    pub fn same_as(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Named { .. }, Self::Fn { .. } | Self::Array { .. } | Self::Ptr { .. }) => false,
            (named @ Self::Named { .. }, Self::Var { type_: lock }) => match lock.try_read() {
                Ok(type_var) => type_var.deref().same_as_other_type(named),
                Err(_) => unreachable!(),
            },
            (
                // TODO: bring them back when we have a better implementation
                // for imports
                Self::Named {
                    // package,
                    // module,
                    name,
                    arguments,
                },
                Self::Named {
                    // package: other_package,
                    // module: other_module,
                    name: other_name,
                    arguments: other_arguments,
                },
            ) => {
                // package == other_package
                // && module == other_module
                // &&
                name == other_name
                    && arguments.len() == other_arguments.len()
                    && arguments
                        .iter()
                        .zip(other_arguments)
                        .all(|(one, other)| one.same_as(other))
            }
            (Self::Fn { .. }, Self::Named { .. } | Self::Array { .. } | Self::Ptr { .. }) => false,
            (function @ Self::Fn { .. }, Self::Var { type_: lock }) => match lock.try_read() {
                Ok(type_var) => type_var.deref().same_as_other_type(function),
                Err(_) => unreachable!(),
            },
            (
                Self::Fn { arguments, return_ },
                Self::Fn {
                    arguments: other_arguments,
                    return_: other_return,
                },
            ) => {
                arguments.len() == other_arguments.len()
                    && arguments
                        .iter()
                        .zip(other_arguments)
                        .all(|(one, other)| one.same_as(other))
                    && return_.same_as(other_return)
            }
            (Type::Var { type_: lock }, other) => match lock.try_read() {
                Ok(type_var) => type_var.deref().same_as_other_type(other),
                Err(_) => unreachable!(),
            },
            (Self::Array { .. }, Self::Named { .. } | Self::Fn { .. } | Self::Ptr { .. }) => false,
            (array @ Self::Array { .. }, Type::Var { type_: lock }) => match lock.try_read() {
                Ok(type_var) => type_var.deref().same_as_other_type(array),
                Err(_) => unreachable!(),
            },
            (
                Self::Array { type_, size },
                Self::Array {
                    type_: other_type,
                    size: other_size,
                },
            ) => type_.same_as(other_type) && size == other_size,
            (Self::Ptr { .. }, Self::Named { .. } | Self::Fn { .. } | Self::Array { .. }) => false,
            (ptr @ Self::Ptr { .. }, Type::Var { type_: lock }) => match lock.try_read() {
                Ok(type_var) => type_var.deref().same_as_other_type(ptr),
                Err(_) => unreachable!(),
            },
            (
                Self::Ptr { type_, .. },
                Self::Ptr {
                    type_: other_type, ..
                },
            ) => type_.same_as(other_type),
        }
    }

    pub fn fn_arity(&self) -> Option<usize> {
        match self {
            Self::Fn { arguments, .. } => Some(arguments.len()),
            Self::Ptr { type_, .. } => type_.fn_arity(),
            _ => None,
        }
    }

    pub fn is_unbound(&self) -> bool {
        match self {
            Self::Var { type_: lock } => match lock.try_read() {
                Ok(type_var) => type_var.deref().is_unbound(),
                Err(_) => unreachable!(),
            },
            _ => false,
        }
    }

    pub fn is_variable(&self) -> bool {
        match self {
            Self::Var { type_: lock } => match lock.try_read() {
                Ok(type_var) => type_var.deref().is_variable(),
                Err(_) => unreachable!(),
            },
            _ => false,
        }
    }

    pub fn is_bool(&self) -> bool {
        match self {
            Self::Named { name, .. } => *name == static_symbol!("bool"),
            Self::Var { type_: lock } => match lock.try_read() {
                Ok(type_var) => type_var.deref().is_bool(),
                Err(_) => unreachable!(),
            },
            _ => false,
        }
    }

    pub fn is_int(&self) -> bool {
        match self {
            Self::Named { name, .. } => matches!(
                name.as_str(),
                "u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64"
            ),

            Self::Var { type_: lock } => match lock.try_read() {
                Ok(type_var) => type_var.deref().is_int(),
                Err(_) => unreachable!(),
            },
            _ => false,
        }
    }

    pub fn is_float(&self) -> bool {
        match self {
            Self::Named { name, .. } => matches!(name.as_str(), "f32" | "f64"),
            Self::Var { type_: lock } => match lock.try_read() {
                Ok(type_var) => type_var.deref().is_float(),
                Err(_) => unreachable!(),
            },
            _ => false,
        }
    }

    pub fn is_number(&self) -> bool {
        match self {
            Self::Named { name, .. } => matches!(
                name.as_str(),
                "u8" | "u16"
                    | "u32"
                    | "u64"
                    | "usize"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "f32"
                    | "f64"
            ),
            Self::Var { type_: lock } => match lock.try_read() {
                Ok(type_var) => type_var.deref().is_number(),
                Err(_) => unreachable!(),
            },
            _ => false,
        }
    }

    pub fn is_str(&self) -> bool {
        match self {
            Self::Named { name, .. } => *name == static_symbol!("str"),
            Self::Var { type_: lock } => match lock.try_read() {
                Ok(type_var) => type_var.deref().is_str(),
                Err(_) => unreachable!(),
            },
            _ => false,
        }
    }

    pub fn is_func(&self) -> bool {
        match self {
            Self::Fn { .. } => true,
            Self::Var { type_: lock } => match lock.try_read() {
                Ok(type_var) => type_var.deref().is_func(),
                Err(_) => unreachable!(),
            },
            _ => false,
        }
    }

    pub fn is_void(&self) -> bool {
        match self {
            Self::Named { name, .. } => *name == static_symbol!("void"),
            Self::Var { type_: lock } => match lock.try_read() {
                Ok(type_var) => type_var.deref().is_void(),
                Err(_) => unreachable!(),
            },
            _ => false,
        }
    }

    pub fn is_aggregate(&self) -> bool {
        match self {
            Self::Named { name, .. } => !matches!(
                name.as_str(),
                "str"
                    | "void"
                    | "bool"
                    | "u8"
                    | "u16"
                    | "u32"
                    | "u64"
                    | "usize"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "f32"
                    | "f64"
            ),
            Self::Array { .. } => true,
            Self::Var { type_: lock } => match lock.try_read() {
                Ok(type_var) => type_var.deref().is_aggregate(),
                Err(_) => unreachable!(),
            },
            _ => false,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Named { name, .. } => write!(formatter, "{}", name),
            Self::Fn { arguments, return_ } => {
                for argument in arguments {
                    write!(formatter, "{} -> ", argument)?;
                }
                write!(formatter, "{}", return_)
            }
            Self::Var { type_ } => type_.read().unwrap().fmt(formatter),
            Self::Array { type_, size } => write!(formatter, "[{}; {}]", type_, size),
            Self::Ptr { type_, .. } => write!(formatter, "*{}", type_),
        }
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.same_as(other)
    }
}

unsafe impl Send for Type {}
unsafe impl Sync for Type {}

/// Unify two types that should be the same.
/// Any unbound type variables will be linked to the other type as they are the
/// same.
///
/// It two types are found to not be the same an error is returned.
pub fn unify(unified: Typed, expected: Typed) -> Result<(), Diagnostic<FileId>> {
    // We don't want to unify the link, we want to unify the type behind the
    // link.
    let unified = collapse_links(unified);

    if let Type::Var { type_: lock } = unified.deref() {
        let type_var = {
            match lock.try_read() {
                Ok(type_var) => type_var.clone(),
                Err(_) => unreachable!(),
            }
        };

        return match type_var {
            TypeVar::Link { type_: t } => unify(t, expected),

            TypeVar::Unbound { id } => {
                unify_unbound_type(&expected, id)?;

                *lock.write().unwrap() = TypeVar::Link {
                    type_: expected.clone(),
                };

                Ok(())
            }

            TypeVar::Generic { id } => {
                if let Type::Var { type_: lock } = expected.deref()
                    && lock.read().unwrap().is_unbound()
                {
                    *lock.write().unwrap() = TypeVar::Generic { id };
                    return Ok(());
                }

                Err(Diagnostic::error().with_message("could not unify"))
            }
        };
    }

    if let Type::Var { type_: lock } = expected.deref() {
        *lock.write().unwrap() = TypeVar::Link {
            type_: unified.clone(),
        };
        return Ok(());
    }

    match (unified.deref(), expected.deref()) {
        (
            Type::Named {
                name: n1,
                arguments: arguments1,
            },
            Type::Named {
                name: n2,
                arguments: arguments2,
            },
        ) if n1 == n2 && arguments1.len() == arguments2.len() => {
            for (a, b) in arguments1.iter().zip(arguments2) {
                unify(a.clone(), b.clone())?;
            }
            Ok(())
        }
        (
            Type::Fn {
                arguments: arguments1,
                return_: return1,
                ..
            },
            Type::Fn {
                arguments: arguments2,
                return_: return2,
                ..
            },
        ) => {
            if arguments1.len() != arguments2.len() {
                return Err(Diagnostic::error()
                    .with_message("functions don't have the same number of arguments"));
            }

            for (unified, expected) in arguments1.iter().zip(arguments2) {
                unify(unified.clone(), expected.clone())?;
            }

            unify(return1.clone(), return2.clone())
        }
        (
            Type::Ptr { type_: unified, .. },
            Type::Ptr {
                type_: expected, ..
            },
        ) => unify(unified.clone(), expected.clone()),

        _ => {
            Err(Diagnostic::error().with_message(format!("expected {}, got {}", unified, expected)))
        }
    }
}

fn unify_unbound_type(type_: &Type, own_id: u64) -> Result<(), Diagnostic<FileId>> {
    match type_ {
        Type::Named { arguments, .. } => {
            for argument in arguments {
                unify_unbound_type(argument, own_id)?;
            }
            Ok(())
        }

        Type::Var { type_: lock } => match lock.try_read() {
            Ok(type_var) => match type_var.deref() {
                TypeVar::Link { type_, .. } => unify_unbound_type(type_.deref(), own_id),

                TypeVar::Unbound { id } => {
                    if *id == own_id {
                        Err(Diagnostic::error().with_message("recursive type"))
                    } else {
                        Ok(())
                    }
                }

                TypeVar::Generic { .. } => Ok(()),
            },
            Err(_) => unreachable!(),
        },

        Type::Fn { arguments, return_ } => {
            for argument in arguments {
                unify_unbound_type(argument, own_id)?;
            }
            unify_unbound_type(return_, own_id)
        }

        Type::Array { type_, .. } | Type::Ptr { type_, .. } => unify_unbound_type(type_, own_id),
    }
}

pub fn generalize(t: Typed) -> Typed {
    match t.deref() {
        Type::Var { type_: lock } => match lock.try_read() {
            Ok(type_var) => match type_var.deref() {
                TypeVar::Unbound { id } => generic_var(*id),
                TypeVar::Link { type_ } => generalize(type_.clone()),
                TypeVar::Generic { .. } => Arc::new(Type::Var {
                    type_: lock.clone(),
                }),
            },
            Err(_) => unreachable!(),
        },

        Type::Named { name, arguments } => {
            let arguments = arguments
                .iter()
                .map(|type_| generalize(type_.clone()))
                .collect();
            named(*name, arguments)
        }

        Type::Fn { arguments, return_ } => fn_(
            arguments
                .iter()
                .map(|type_| generalize(type_.clone()))
                .collect(),
            generalize(return_.clone()),
        ),

        Type::Array { type_, size } => array(generalize(type_.clone()), *size),

        Type::Ptr { type_, unchecked } => ptr(generalize(type_.clone()), *unchecked),
    }
}
