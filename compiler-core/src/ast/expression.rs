use crate::{
    ast::{
        Location, Typed, TypedExpr, UntypedExpr, UntypedStmt, identifier::Identifier,
        statement::Stmt,
    },
    lexer::{Source, Token},
};
use std::boxed::Box;

#[derive(Debug)]
pub struct Expr<T> {
    kind: ExprKind<T>,
    source: Source,
    type_: T,
}

impl<T> Expr<T> {
    pub fn kind(&self) -> &ExprKind<T> {
        &self.kind
    }

    pub fn type_(&self) -> &T {
        &self.type_
    }
}

impl UntypedExpr {
    pub fn new(kind: ExprKind<()>, source: Source) -> Self {
        Self {
            kind,
            source,
            type_: (),
        }
    }
}

impl TypedExpr {
    pub fn new(kind: ExprKind<Typed>, source: Source, type_: Typed) -> Self {
        Self {
            kind,
            source,
            type_,
        }
    }
}

impl<T> Location for Expr<T> {
    fn source(&self) -> &Source {
        &self.source
    }
}

#[derive(Debug)]
pub enum ExprKind<T> {
    Unary {
        // -right
        operator: Token,
        right: Box<Expr<T>>,
    },
    Binary {
        // left + right
        left: Box<Expr<T>>,
        operator: Token,
        right: Box<Expr<T>>,
    },
    Get {
        // left.right
        left: Box<Expr<T>>,
        right: Identifier,
    },
    Index {
        // object[index]
        object: Box<Expr<T>>,
        index: Box<Expr<T>>,
    },
    Call {
        // callee(..arguments)
        callee: Box<Expr<T>>,
        arguments: Vec<Expr<T>>,
    },
    CreateArray {
        name: Identifier,
        size: Option<Box<Expr<T>>>,
        fields: Vec<Expr<T>>,
    },
    CreateStruct {
        name: Identifier,
        fields: Vec<(Identifier, Expr<T>)>,
    },
    If {
        condition: Box<Expr<T>>,
        if_branch: Box<Stmt<T>>,
        else_branch: Option<Box<Stmt<T>>>,
    },
    Literal(Token),
    Variable {
        name: Identifier,
    },
}

pub trait UntypedExprVisitor<'a> {
    type Output;

    fn visit_expr(&mut self, expr: &'a UntypedExpr) -> Self::Output {
        match expr.kind() {
            ExprKind::Unary { operator, right } => self.visit_unary(operator, right),
            ExprKind::Binary {
                left,
                operator,
                right,
            } => self.visit_binary(left, operator, right),
            ExprKind::Get { left, right } => self.visit_get(left, right),
            ExprKind::Index { object, index } => self.visit_index(object, index),
            ExprKind::Call { callee, arguments } => self.visit_call(callee, arguments),
            ExprKind::CreateArray { name, size, fields } => {
                self.visit_create_array(name, size, fields)
            }
            ExprKind::CreateStruct { name, fields } => self.visit_create_struct(name, fields),
            ExprKind::If {
                condition,
                if_branch,
                else_branch,
            } => self.visit_if(condition, if_branch, else_branch),
            ExprKind::Literal(value) => self.visit_literal(value),
            ExprKind::Variable { name } => self.visit_variable(name),
        }
    }

    fn visit_unary(&mut self, operator: &'a Token, right: &'a Box<UntypedExpr>) -> Self::Output;

    fn visit_binary(
        &mut self,
        left: &'a Box<UntypedExpr>,
        operator: &'a Token,
        right: &'a Box<UntypedExpr>,
    ) -> Self::Output;

    fn visit_get(&mut self, left: &'a Box<UntypedExpr>, right: &'a Identifier) -> Self::Output;

    fn visit_index(
        &mut self,
        object: &'a Box<UntypedExpr>,
        index: &'a Box<UntypedExpr>,
    ) -> Self::Output;

    fn visit_call(
        &mut self,
        callee: &'a Box<UntypedExpr>,
        arguments: &'a [UntypedExpr],
    ) -> Self::Output;

    fn visit_create_array(
        &mut self,
        name: &'a Identifier,
        size: &'a Option<Box<UntypedExpr>>,
        fields: &'a [UntypedExpr],
    ) -> Self::Output;

    fn visit_create_struct(
        &mut self,
        name: &'a Identifier,
        fields: &'a [(Identifier, UntypedExpr)],
    ) -> Self::Output;

    fn visit_if(
        &mut self,
        condition: &'a Box<UntypedExpr>,
        if_branch: &'a Box<UntypedStmt>,
        else_branch: &'a Option<Box<UntypedStmt>>,
    ) -> Self::Output;

    fn visit_literal(&mut self, value: &'a Token) -> Self::Output;

    fn visit_variable(&mut self, name: &'a Identifier) -> Self::Output;
}

pub trait TypedExprVisitor<'a> {
    type Type;
    type Output;

    fn visit_expr(&mut self, expr: &'a Expr<Self::Type>) -> Self::Output {
        match expr.kind() {
            ExprKind::Unary { operator, right } => self.visit_unary(operator, right, expr.type_()),
            ExprKind::Binary {
                left,
                operator,
                right,
            } => self.visit_binary(left, operator, right, expr.type_()),
            ExprKind::Get { left, right } => self.visit_get(left, right, expr.type_()),
            ExprKind::Index { object, index } => self.visit_index(object, index, expr.type_()),
            ExprKind::Call { callee, arguments } => {
                self.visit_call(callee, arguments, expr.type_())
            }
            ExprKind::CreateArray { name, size, fields } => {
                self.visit_create_array(name, size, fields, expr.type_())
            }
            ExprKind::CreateStruct { name, fields } => {
                self.visit_create_struct(name, fields, expr.type_())
            }
            ExprKind::If {
                condition,
                if_branch,
                else_branch,
            } => self.visit_if(condition, if_branch, else_branch, expr.type_()),
            ExprKind::Literal(value) => self.visit_literal(value, expr.type_()),
            ExprKind::Variable { name } => self.visit_variable(name, expr.type_()),
        }
    }

    fn visit_unary(
        &mut self,
        operator: &'a Token,
        right: &'a Box<Expr<Self::Type>>,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_binary(
        &mut self,
        left: &'a Box<Expr<Self::Type>>,
        operator: &'a Token,
        right: &'a Box<Expr<Self::Type>>,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_get(
        &mut self,
        left: &'a Box<Expr<Self::Type>>,
        right: &'a Identifier,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_index(
        &mut self,
        object: &'a Box<Expr<Self::Type>>,
        index: &'a Box<Expr<Self::Type>>,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_call(
        &mut self,
        callee: &'a Box<Expr<Self::Type>>,
        arguments: &'a [Expr<Self::Type>],
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_create_array(
        &mut self,
        name: &'a Identifier,
        size: &'a Option<Box<Expr<Self::Type>>>,
        fields: &'a [Expr<Self::Type>],
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_create_struct(
        &mut self,
        name: &'a Identifier,
        fields: &'a [(Identifier, Expr<Self::Type>)],
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_if(
        &mut self,
        condition: &'a Box<Expr<Self::Type>>,
        if_branch: &'a Box<Stmt<Self::Type>>,
        else_branch: &'a Option<Box<Stmt<Self::Type>>>,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_literal(&mut self, value: &'a Token, type_: &'a Self::Type) -> Self::Output;

    fn visit_variable(&mut self, name: &'a Identifier, type_: &'a Self::Type) -> Self::Output;
}
