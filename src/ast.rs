use std::rc::Rc;

impl Ast {
    pub fn r(self) -> Rc<Self> {
        Rc::new(self)
    }
}

#[derive(Debug)]
pub enum PrimitiveTypes {
    String,
    I64,
    I32,
    F64,
    F32,
    Custom(String),
}
#[derive(Debug)]
pub enum Primitive {
    String(String),
    Number(i64),
}

#[derive(Debug)]
pub enum BinaryOp {
    Equal,
    NotEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug)]
pub enum UnaryOp {
    Not,
}

#[derive(Debug)]
pub enum Ast {
    Primitive,
    Id(Id),
    Composit {
        name: String,
        fields: Vec<Type>,
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
        callee: Id,
        args: Vec<Ast>,
    },
    Return {
        term: Rc<Ast>,
    },
    Block {
        terms: Vec<Ast>,
    },
    If {
        conditional: Rc<Ast>,
        consequence: Rc<Ast>,
        alternative: Rc<Ast>,
    },
    Function {
        name: Id,
        parameters: Vec<(String, Type)>,
        body: Rc<Ast>,
    },
    Var {
        name: String,
        value: Rc<Ast>,
        r#type: Type,
    },
    Assignment {
        name: String,
        value: Rc<Ast>,
    },
    While {
        conditional: Rc<Ast>,
        body: Rc<Ast>,
    },
}
#[derive(Debug)]
pub struct Type {
    pub name: String,
    pub r#type: PrimitiveTypes,
}

#[derive(Debug)]
pub struct Id {
    pub id: String,
}

impl Id {
    pub fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}
