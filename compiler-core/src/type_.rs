use std::{cell::RefCell, sync::Arc};
use symbol_table::{GlobalSymbol, static_symbol};

pub mod type_var;

pub use type_var::TypeVar;

#[derive(Debug)]
pub enum Type {
    /// A named type such as i32, f64 or a custom type such as Vec.
    /// The type can take other types as generic arguments
    Named {
        // TODO: bring them back when we have a better implementation for imports
        // package: GlobalSymbol,
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

    /// Reference or pointer to another type
    Ptr {
        /// Raw pointers (*) are unchecked, references are checked (&)
        unchecked: bool,
        /// The referenced type
        type_: Arc<Type>,
    },
}

impl Type {
    #[must_use]
    pub fn same_as(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Named { .. }, Self::Fn { .. } | Self::Ptr { .. }) => false,
            (named @ Self::Named { .. }, Self::Var { type_ }) => {
                type_.as_ref().borrow().same_as_other_type(named)
            }
            (
                // TODO: bring them back when we have a better implementation for imports
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
            (Self::Fn { .. }, Self::Named { .. } | Self::Ptr { .. }) => false,
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
            (Self::Ptr { .. }, Self::Named { .. } | Self::Fn { .. }) => false,
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
            Self::Named { name, .. } => matches!(name.as_str(), "u8" | "u16" | "u32" | "u128"),
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
}
