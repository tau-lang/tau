use crate::error::FileId;
use codespan_reporting::diagnostic::Diagnostic;
use std::{cell::RefCell, fmt, ops::Deref, sync::Arc};
use symbol_table::{GlobalSymbol, static_symbol};

pub mod declaration;
pub mod environment;
pub mod expression;
pub mod prelude;
pub mod statement;
pub mod type_var;

pub use crate::type_::prelude::*;
pub use type_var::{TypeVar, collapse_links};

#[derive(Clone, Debug, PartialEq)]
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
        type_: Arc<RefCell<TypeVar>>,
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
            (named @ Self::Named { .. }, Self::Var { type_ }) => {
                type_.as_ref().borrow().same_as_other_type(named)
            }
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
            (function @ Self::Fn { .. }, Self::Var { type_ }) => {
                type_.as_ref().borrow().same_as_other_type(function)
            }
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
            (Type::Var { type_ }, other) => type_.as_ref().borrow().same_as_other_type(other),
            (Self::Array { .. }, Self::Named { .. } | Self::Fn { .. } | Self::Ptr { .. }) => false,
            (array @ Self::Array { .. }, Type::Var { type_ }) => {
                type_.as_ref().borrow().same_as_other_type(array)
            }
            (
                Self::Array { type_, size },
                Self::Array {
                    type_: other_type,
                    size: other_size,
                },
            ) => type_.same_as(other_type) && size == other_size,
            (Self::Ptr { .. }, Self::Named { .. } | Self::Fn { .. } | Self::Array { .. }) => false,
            (ptr @ Self::Ptr { .. }, Type::Var { type_ }) => {
                type_.as_ref().borrow().same_as_other_type(ptr)
            }
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
            Self::Var { type_ } => type_.borrow().is_unbound(),
            _ => false,
        }
    }

    pub fn is_variable(&self) -> bool {
        match self {
            Self::Var { type_ } => type_.borrow().is_variable(),
            _ => false,
        }
    }

    pub fn is_bool(&self) -> bool {
        match self {
            Self::Named { name, .. } => *name == static_symbol!("bool"),
            Self::Var { type_ } => type_.borrow().is_bool(),
            _ => false,
        }
    }

    pub fn is_int(&self) -> bool {
        match self {
            Self::Named { name, .. } => matches!(
                name.as_str(),
                "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64"
            ),
            Self::Var { type_ } => type_.borrow().is_int(),
            _ => false,
        }
    }

    pub fn is_float(&self) -> bool {
        match self {
            Self::Named { name, .. } => matches!(name.as_str(), "f32" | "f64"),
            Self::Var { type_ } => type_.borrow().is_float(),
            _ => false,
        }
    }

    pub fn is_number(&self) -> bool {
        match self {
            Self::Named { name, .. } => matches!(
                name.as_str(),
                "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "f32" | "f64"
            ),
            Self::Var { type_ } => type_.borrow().is_float(),
            _ => false,
        }
    }

    pub fn is_str(&self) -> bool {
        match self {
            Self::Named { name, .. } => *name == static_symbol!("str"),
            Self::Var { type_ } => type_.borrow().is_int(),
            _ => false,
        }
    }

    pub fn is_func(&self) -> bool {
        match self {
            Self::Fn { .. } => true,
            Self::Var { type_ } => type_.borrow().is_func(),
            _ => false,
        }
    }

    pub fn is_void(&self) -> bool {
        match self {
            Self::Named { name, .. } => *name == static_symbol!("void"),
            Self::Var { type_ } => type_.borrow().is_void(),
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
            Self::Var { type_ } => write!(formatter, "{:?}", type_),
            Self::Array { type_, size } => write!(formatter, "[{}; {}]", type_, size),
            Self::Ptr { type_, .. } => write!(formatter, "*{}", type_),
        }
    }
}

/// Unify two types that should be the same.
/// Any unbound type variables will be linked to the other type as they are the
/// same.
///
/// It two types are found to not be the same an error is returned.
pub fn unify(t1: Arc<Type>, t2: Arc<Type>) -> Result<(), Diagnostic<FileId>> {
    if t1 == t2 {
        return Ok(());
    }

    // Collapse right hand side type links. Left hand side will be collapsed in
    // the next block.
    if let Type::Var { type_ } = t2.deref()
        && let TypeVar::Link { type_ } = type_.borrow().deref()
    {
        return unify(t1, type_.clone());
    }

    if let Type::Var { type_ } = t1.deref() {
        return match type_.borrow().deref() {
            TypeVar::Link { type_: t } => {
                unify(t.clone(), t2)?;

                // Note that type_ is always a Link in this branch.
                // unify may replace t's inner value with another link
                // (See the Action::Link branch just above)
                // This can cause the compiler to build up an ever-growing chain
                // of links. Therefore, we try to collapse the
                // links. However, the RefCell in type_
                // may already be borrowed by collapsing the links in t2 at the
                // start of the function, in which case accept
                // the extra link.
                if let Ok(mut type_) = type_.try_borrow_mut() {
                    *type_ = TypeVar::Link {
                        type_: collapse_links(t.clone()),
                    }
                }
                Ok(())
            }

            TypeVar::Unbound { id } => {
                unify_unbound_type(&t2, *id)?;
                {
                    *type_.borrow_mut() = TypeVar::Link { type_: t2.clone() };
                    Ok(())
                }
            }

            TypeVar::Generic { id } => {
                if let Type::Var { type_ } = t2.deref()
                    && type_.borrow().is_unbound()
                {
                    *type_.borrow_mut() = TypeVar::Generic { id: *id };
                    return Ok(());
                }
                Err(Diagnostic::error().with_message("could not unify"))
            }
        };
    }

    if let Type::Var { .. } = t2.deref() {
        return unify(t2, t1);
    }

    match (t1.deref(), t2.deref()) {
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

            for (t1, t2) in arguments1.iter().zip(arguments2) {
                unify(t1.clone(), t2.clone())?;
            }

            unify(return1.clone(), return2.clone())
        }
        (Type::Ptr { type_: t1, .. }, Type::Ptr { type_: t2, .. }) => unify(t1.clone(), t2.clone()),

        _ => Err(Diagnostic::error().with_message(format!("expected {}, got {}", t1, t2))),
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

        Type::Var { type_ } => match type_.borrow().deref() {
            TypeVar::Link { type_, .. } => unify_unbound_type(type_, own_id),

            TypeVar::Unbound { id } => {
                if id == &own_id {
                    Err(Diagnostic::error().with_message("recursive type"))
                } else {
                    Ok(())
                }
            }

            TypeVar::Generic { .. } => Ok(()),
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

pub fn generalise(t: Arc<Type>) -> Arc<Type> {
    match t.deref() {
        Type::Var { type_ } => match type_.borrow().deref() {
            TypeVar::Unbound { id } => generic_var(*id),
            TypeVar::Link { type_ } => generalise(type_.clone()),
            TypeVar::Generic { .. } => Arc::new(Type::Var {
                type_: type_.clone(),
            }),
        },

        Type::Named { name, arguments } => {
            let arguments = arguments
                .iter()
                .map(|type_| generalise(type_.clone()))
                .collect();
            Arc::new(Type::Named {
                name: name.clone(),
                arguments,
            })
        }

        Type::Fn { arguments, return_ } => Arc::new(Type::Fn {
            arguments: arguments
                .iter()
                .map(|type_| generalise(type_.clone()))
                .collect(),
            return_: generalise(return_.clone()),
        }),

        Type::Array { type_, size } => Arc::new(Type::Array {
            size: *size,
            type_: generalise(type_.clone()),
        }),

        Type::Ptr { type_, unchecked } => Arc::new(Type::Ptr {
            unchecked: *unchecked,
            type_: generalise(type_.clone()),
        }),
    }
}
