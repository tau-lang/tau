use crate::{
    ast::{Location, Typed, expression::Expr, identifier::Identifier},
    lexer::Source,
    type_::Type,
};
use std::{boxed::Box, sync::Arc};

#[derive(Debug)]
pub struct Stmt<T> {
    kind: StmtKind<T>,
    source: Source,
}

impl<T> Stmt<T> {
    pub fn new(kind: StmtKind<T>, source: Source) -> Self {
        Stmt { kind, source }
    }

    pub fn kind(&self) -> &StmtKind<T> {
        &self.kind
    }
}

impl<T> Location for Stmt<T> {
    fn source(&self) -> &Source {
        &self.source
    }
}

#[derive(Debug)]
pub enum StmtKind<T> {
    Block {
        statements: Vec<Box<Stmt<T>>>,
    },
    Let {
        name: Identifier,
        declared: Option<Arc<Type>>,
        initializer: Expr<T>,
    },
    Return {
        value: Expr<T>,
    },
    Break,
    While {
        condition: Expr<T>,
        body: Box<Stmt<T>>,
    },
    ExprStmt(Expr<T>),
}

pub trait StmtVisitor<'a> {
    type Type;
    type Output;

    fn visit_stmt(&mut self, stmt: &'a Stmt<Self::Type>) -> Self::Output {
        match stmt.kind() {
            StmtKind::Block { statements } => self.visit_block(statements),
            StmtKind::Let {
                name,
                declared,
                initializer,
            } => self.visit_let(name, declared, initializer),
            StmtKind::Return { value } => self.visit_return(value),
            StmtKind::Break => self.visit_break(),
            StmtKind::While { condition, body } => self.visit_while(condition, body),
            StmtKind::ExprStmt(expr) => self.visit_expr_stmt(expr),
        }
    }

    fn visit_block(&mut self, statements: &'a [Box<Stmt<Self::Type>>]) -> Self::Output;

    fn visit_let(
        &mut self,
        name: &'a Identifier,
        declared: &'a Option<Typed>,
        initializer: &'a Expr<Self::Type>,
    ) -> Self::Output;

    fn visit_return(&mut self, value: &'a Expr<Self::Type>) -> Self::Output;

    fn visit_break(&mut self) -> Self::Output;

    fn visit_while(
        &mut self,
        condition: &'a Expr<Self::Type>,
        body: &'a Box<Stmt<Self::Type>>,
    ) -> Self::Output;

    fn visit_expr_stmt(&mut self, expr: &'a Expr<Self::Type>) -> Self::Output;
}
