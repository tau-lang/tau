use crate::type_::Type;
use std::{ops::Deref, sync::Arc};

#[derive(Clone, Debug, PartialEq)]
pub enum TypeVar {
    /// Unbound variable of one specific type, but we don't know the type yet.
    Unbound { id: u64 },
    /// Variable where we worked out the type.
    Link { type_: Arc<Type> },
    /// Generic variable where we can not know the type.
    Generic { id: u64 },
}

macro_rules! follow_link {
    ($type_var:expr, $func:ident, $default:expr) => {
        match $type_var {
            crate::type_::TypeVar::Link { type_ } => type_.$func(),
            _ => $default,
        }
    };
    ($type_var:expr, $func:ident) => {
        follow_link!($type_var, $func, false)
    };
}

impl TypeVar {
    #[must_use]
    pub fn same_as_other_type(&self, other: &Type) -> bool {
        match (self, other) {
            (Self::Unbound { .. }, _) => true,
            (TypeVar::Link { type_ }, other) => type_.same_as(other),
            (
                Self::Generic { .. },
                Type::Named { .. } | Type::Fn { .. } | Type::Array { .. } | Type::Ptr { .. },
            ) => false,
            (one @ TypeVar::Generic { .. }, Type::Var { type_ }) => {
                one.same_as(&type_.as_ref().borrow())
            }
        }
    }

    #[must_use]
    pub fn same_as(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unbound { .. }, _) | (_, Self::Unbound { .. }) => true,
            (Self::Link { type_ }, Self::Link { type_: other_type }) => type_.same_as(other_type),
            (Self::Link { type_ }, Self::Generic { .. }) => other.same_as_other_type(type_),
            (Self::Generic { id }, Self::Generic { id: other_id }) => id == other_id,
            (generic @ Self::Generic { .. }, Self::Link { type_ }) => {
                generic.same_as_other_type(type_)
            }
        }
    }

    pub fn is_unbound(&self) -> bool {
        match self {
            Self::Unbound { .. } => true,
            Self::Link { type_ } => type_.is_unbound(),
            Self::Generic { .. } => false,
        }
    }

    pub fn is_variable(&self) -> bool {
        follow_link!(self, is_variable, true)
    }

    pub fn is_func(&self) -> bool {
        follow_link!(self, is_func)
    }

    pub fn is_bool(&self) -> bool {
        follow_link!(self, is_bool)
    }

    pub fn is_int(&self) -> bool {
        follow_link!(self, is_int)
    }

    pub fn is_float(&self) -> bool {
        follow_link!(self, is_float)
    }

    pub fn is_number(&self) -> bool {
        follow_link!(self, is_number)
    }

    pub fn is_str(&self) -> bool {
        follow_link!(self, is_str)
    }

    pub fn is_void(&self) -> bool {
        follow_link!(self, is_void)
    }
}

pub fn collapse_links(t: Arc<Type>) -> Arc<Type> {
    if let Type::Var { type_ } = t.deref()
        && let TypeVar::Link { type_ } = type_.borrow().deref()
    {
        return collapse_links(type_.clone());
    }
    t
}
