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
    Create {
        struct_name: Token,
        fields: Vec<(Token, Rc<Expr>)>,
    },
    If {
        condition: Rc<Expr>,
        if_branch: Rc<Stmt>,
        else_branch: Option<Rc<Stmt>>,
    },
    Literal(Token),
    Variable(Token),
}

#[derive(Debug)]
pub enum Stmt {
    Block {
        statements: Vec<Rc<Stmt>>,
    },
    Let {
        name: Token,
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
        initializer: Rc<Stmt>,
        condition: Expr,
        increment: Expr,
        body: Rc<Stmt>,
    },
    Expr(Expr),
}

#[derive(Debug)]
pub enum Decl {
    Import(Token),
    Struct {
        name: Token,
        fields: Vec<(Token, Token)>,
    },
    Function {
        name: Token,
        return_type: Token,
        params: Vec<(Token, Token)>,
        body: Stmt,
    },
    Const {
        name: Token,
        var_type: Token,
        initializer: Expr,
    },
}
