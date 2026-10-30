use crate::{ast::Location, lexer::Source};
use std::fmt::{self, Debug};

#[derive(Debug)]
pub enum BinaryOp {
    // (number -> number) -> number
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // number <- (number -> number)
    Set,
    SetAdd,
    SetSub,
    SetMul,
    SetDiv,

    // forall a: (a -> a) -> bool
    Eq,
    Neq,

    // (number -> number) -> bool
    Low,
    Leq,
    Gre,
    Geq,

    // (bool -> bool) -> bool
    And,
    Or,
}

#[derive(Debug)]
pub enum UnaryOp {
    // number -> number
    Neg,

    // bool -> bool
    Not,

    // a -> *a
    Ref,

    // *a -> a
    Deref,
}

#[derive(Debug)]
pub enum Literal {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

pub struct ExprOrigin<O> {
    kind: O,
    source: Source,
}

impl<O> ExprOrigin<O> {
    pub fn new(kind: O, source: Source) -> Self {
        Self { kind, source }
    }

    pub fn kind(&self) -> &O {
        &self.kind
    }
}

impl<O> Location for ExprOrigin<O> {
    fn source(&self) -> Source {
        self.source
    }
}

impl<O> Debug for ExprOrigin<O>
where
    O: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ExprOrigin {{ kind: {:?}, source: {:?} }}",
            self.kind, self.source
        )
    }
}
