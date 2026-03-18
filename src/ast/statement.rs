use crate::{
    ast::{expression::Expr, identifier::Identifier},
    lexer::Source,
    typing::TypeCell,
};
use std::rc::Rc;

#[derive(Debug)]
pub struct Stmt {
    kind: StmtType,
    source: Source,
}

impl Stmt {
    pub fn new(source: Source, kind: StmtType) -> Self {
        Stmt { kind, source }
    }
    pub fn source(&self) -> Source {
        self.source.clone()
    }
    pub fn kind(&self) -> &StmtType {
        &self.kind
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum StmtType {
    Block {
        statements: Vec<Rc<Stmt>>,
    },
    Let {
        name: Identifier,
        var_type: TypeCell,
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
    ExprStmt(Expr),
}

pub trait StmtVisitor<'a, T> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) -> T {
        match stmt.kind() {
            StmtType::Block { statements } => self.visit_block(statements),
            StmtType::Let {
                name,
                var_type,
                initializer,
            } => self.visit_let(name, var_type, initializer),
            StmtType::Return { value } => self.visit_return(value),
            StmtType::Break => self.visit_break(),
            StmtType::While { condition, body } => self.visit_while(condition, body),
            StmtType::For {
                initializer,
                condition,
                increment,
                body,
            } => self.visit_for(initializer, condition, increment, body),
            StmtType::ExprStmt(expr) => self.visit_expr_stmt(expr),
        }
    }

    fn visit_block(&mut self, statements: &'a [Rc<Stmt>]) -> T;

    fn visit_let(
        &mut self,
        name: &'a Identifier,
        var_type: &'a TypeCell,
        initializer: &'a Expr,
    ) -> T;

    fn visit_return(&mut self, value: &'a Expr) -> T;

    fn visit_break(&mut self) -> T;

    fn visit_while(&mut self, condition: &'a Expr, body: &'a Rc<Stmt>) -> T;

    fn visit_for(
        &mut self,
        initializer: &'a Rc<Stmt>,
        condition: &'a Expr,
        increment: &'a Expr,
        body: &'a Rc<Stmt>,
    ) -> T;

    fn visit_expr_stmt(&mut self, expr: &'a Expr) -> T;
}
