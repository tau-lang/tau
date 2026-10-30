use crate::type_::{Type, TypeVar};
use std::{cell::RefCell, sync::Arc};
use symbol_table::{GlobalSymbol, static_symbol};

pub type Untyped = ();
pub type Typed = Arc<Type>;

pub fn fn_(arguments: Vec<Arc<Type>>, return_: Arc<Type>) -> Arc<Type> {
    Arc::new(Type::Fn { return_, arguments })
}

pub fn named(name: GlobalSymbol, arguments: Vec<Arc<Type>>) -> Arc<Type> {
    Arc::new(Type::Named {
        name: name,
        arguments,
    })
}

fn native(name: GlobalSymbol) -> Typed {
    named(name, vec![])
}

pub fn bool() -> Typed {
    native(static_symbol!("bool"))
}

pub fn void() -> Typed {
    native(static_symbol!("void"))
}

/// Any int literal of unknown size.
pub fn int() -> Typed {
    // TODO: return an unsized int
    native(static_symbol!("i64"))
}

/// Any float literal of unknown size.
pub fn float() -> Typed {
    // TODO: return an unsized float
    native(static_symbol!("f64"))
}

pub fn str() -> Typed {
    native(static_symbol!("str"))
}

pub fn ptr(inner: Typed, unchecked: bool) -> Typed {
    Typed::new(Type::Ptr {
        type_: inner,
        unchecked,
    })
}

pub fn array(inner: Typed, size: usize) -> Typed {
    Arc::new(Type::Array { type_: inner, size })
}

pub fn generic_var(id: u64) -> Arc<Type> {
    Arc::new(Type::Var {
        type_: Arc::new(RefCell::new(TypeVar::Generic { id })),
    })
}

pub fn unbound_var(id: u64) -> Arc<Type> {
    Arc::new(Type::Var {
        type_: Arc::new(RefCell::new(TypeVar::Unbound { id })),
    })
}

pub fn link(type_: Arc<Type>) -> Arc<Type> {
    Arc::new(Type::Var {
        type_: Arc::new(RefCell::new(TypeVar::Link { type_ })),
    })
}
