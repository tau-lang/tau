use crate::parser::{error, typechecked};
use miette::SourceOffset;
use std::rc::Rc;

impl Ast {
    pub fn r(self) -> Rc<Self> {
        Rc::new(self)
    }
    pub fn check_types(self) -> Result<Self, error::CheckError> {
        typechecked::typecheck(self)
    }
}

#[derive(Debug, PartialEq)]
pub enum PrimitiveTypes {
    String,
    I64,
    I32,
    F64,
    F32,
    Custom(String),
    Unit,
}
#[derive(Debug, PartialEq)]
pub enum Primitive {
    String(String),
    Int(i64),
    Float(f64),
    Unit,
}

#[derive(Debug, PartialEq)]
pub enum BinaryOp {
    Equal,
    NotEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, PartialEq)]
pub enum UnaryOp {
    Not,
}

#[derive(Debug, PartialEq)]
pub enum Ast {
    Primitive(Primitive),
    Id(Id),
    Imports(Vec<String>),
    CompositDef {
        name: String,
        fields: Vec<Type>,
    },
    CompositConstruction {
        source: SourceOffset,
        values: Vec<(String, Ast)>,
    },
    Enum {
        variants: Vec<Type>,
    },
    UnaryOp {
        op: UnaryOp,
        term: Rc<Ast>,
    },
    BinaryOp {
        op: BinaryOp,
        lhs: Rc<Ast>,
        rhs: Rc<Ast>,
    },
    Call {
        source: SourceOffset,
        callee: Id,
        args: Vec<Ast>,
    },
    Return {
        source: SourceOffset,
        term: Rc<Ast>,
    },
    Block {
        source: SourceOffset,
        terms: Vec<Ast>,
    },
    If {
        conditional: Rc<Ast>,
        consequence: Rc<Ast>,
        alternative: Rc<Ast>,
    },
    Function {
        source: SourceOffset,
        name: Id,
        return_type: PrimitiveTypes,
        parameters: Vec<Type>,
        body: Vec<Ast>,
    },
    Var {
        name: String,
        source: SourceOffset,
        value: Rc<Ast>,
        r#type: PrimitiveTypes,
    },
    Assignment {
        name: String,
        source: SourceOffset,
        value: Rc<Ast>,
    },
    While {
        conditional: Rc<Ast>,
        body: Rc<Ast>,
    },
    Modification {
        what: Id,
        val: Rc<Ast>,
    },
}
#[derive(Debug, PartialEq)]
pub struct Type {
    pub name: String,
    pub r#type: PrimitiveTypes,
}

#[derive(Debug, PartialEq)]
pub struct Id {
    pub id: String,
}

impl Id {
    pub fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}
