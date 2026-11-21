use crate::{ast::Location, lexer::Source};
use std::fmt::{self, Debug};

#[derive(PartialEq, Debug)]
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

#[derive(PartialEq, Debug)]
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

#[derive(PartialEq, Debug)]
pub enum Literal {
    Bool(bool),
    Int(u64),
    Float(f64),
    String(String),
}

pub struct Origin<O> {
    kind: O,
    source: Source,
}

impl<O> Origin<O> {
    pub fn new(kind: O, source: Source) -> Self {
        Self { kind, source }
    }

    pub fn kind(&self) -> &O {
        &self.kind
    }
}

impl<O> Location for Origin<O> {
    fn source(&self) -> Source {
        self.source
    }
}

impl<O> PartialEq for Origin<O>
where
    O: PartialEq,
{
    fn eq(&self, other: &Origin<O>) -> bool {
        self.kind == other.kind && self.source == other.source
    }
}

impl<O> Debug for Origin<O>
where
    O: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Origin {{ kind: {:?}, source: {:?} }}",
            self.kind, self.source
        )
    }
}
