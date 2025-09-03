use crate::lexer::Token;
use std::rc::Rc;

#[derive(Debug)]
pub enum Expr {
    Unary {
        // -right
        operator: Token,
        right: Rc<Expr>,
    },
    Binary {
        // left + right
        left: Rc<Expr>,
        operator: Token,
        right: Rc<Expr>,
    },
    Set {
        // object.name = right
        object: Rc<Expr>,
        name: String,
        operator: Token,
        right: Rc<Expr>,
    },
    Get {
        // object.name
        object: Rc<Expr>,
        name: String,
    },
    Index {
        // object[index]
        object: Rc<Expr>,
        index: Rc<Expr>,
    },
    Call {
        // callee(..arguments)
        callee: Rc<Expr>,
        arguments: Vec<Rc<Expr>>,
    },
    Literal(Token),
    Variable(String),
}

#[derive(Debug)]
pub struct TypeDef {
    var: String,
    var_type: String,
}

#[derive(Debug)]
pub enum Stmt {
    Block {
        statements: Vec<Rc<Stmt>>,
    },
    Struct {
        name: String,
        fields: Vec<TypeDef>,
    },
    Function {
        name: String,
        return_type: String,
        params: Vec<TypeDef>,
        body: Rc<Stmt>,
    },
    If {
        condition: Expr,
        if_branch: Rc<Stmt>,
        else_branch: Option<Rc<Stmt>>,
    },
    Let {
        name: String,
        initializer: Expr,
    },
    Return {
        value: Expr,
    },
    Break,
    While {
        condition: Expr,
        body: Rc<Stmt>,
    },
    For {
        initializer: Expr,
        condition: Expr,
        increment: Expr,
        body: Rc<Stmt>,
    },
}
