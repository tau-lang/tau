use crate::{
    ast::{
        Location, Typed, UntypedExpr, UntypedStmt,
        identifier::Identifier,
        origin::{BinaryOp, ExprOrigin, Literal, UnaryOp},
        statement::Stmt,
    },
    lexer::Source,
};
use std::boxed::Box;

#[derive(Debug)]
pub struct Expr<T> {
    kind: ExprKind<T>,
}

impl<T> Expr<T> {
    pub fn new(kind: ExprKind<T>) -> Self {
        Self { kind }
    }

    pub fn kind(&self) -> &ExprKind<T> {
        &self.kind
    }
}

impl Expr<Typed> {
    pub fn type_(&self) -> &Typed {
        match self.kind() {
            ExprKind::Unary { type_, .. }
            | ExprKind::Binary { type_, .. }
            | ExprKind::Call { type_, .. }
            | ExprKind::IndexArray { type_, .. }
            | ExprKind::AccessStruct { type_, .. }
            | ExprKind::CreateArray { type_, .. }
            | ExprKind::CreateStruct { type_, .. }
            | ExprKind::If { type_, .. }
            | ExprKind::Literal { type_, .. }
            | ExprKind::Variable { type_, .. } => type_,
        }
    }
}

impl<T> Location for Expr<T> {
    fn source(&self) -> Source {
        match self.kind() {
            ExprKind::Call { source, .. }
            | ExprKind::IndexArray { source, .. }
            | ExprKind::CreateArray { source, .. }
            | ExprKind::CreateStruct { source, .. }
            | ExprKind::If { source, .. } => *source,
            ExprKind::Unary {
                operator: start,
                right: end,
                ..
            } => start.source().merge(end.source()),
            ExprKind::Binary {
                left: start,
                right: end,
                ..
            } => start.source().merge(end.source()),
            ExprKind::AccessStruct {
                left: start,
                right: end,
                ..
            } => start.source().merge(end.source()),
            ExprKind::Literal { literal, .. } => literal.source(),
            ExprKind::Variable { name, .. } => name.source(),
        }
    }
}

#[derive(Debug)]
pub enum ExprKind<T> {
    Unary {
        // -right
        operator: ExprOrigin<UnaryOp>,
        right: Box<Expr<T>>,
        type_: T,
    },
    Binary {
        // left + right
        left: Box<Expr<T>>,
        operator: ExprOrigin<BinaryOp>,
        right: Box<Expr<T>>,
        type_: T,
    },
    Call {
        // callee(..arguments)
        callee: Box<Expr<T>>,
        arguments: Vec<Expr<T>>,
        source: Source,
        type_: T,
    },
    IndexArray {
        // object[index]
        object: Box<Expr<T>>,
        index: Box<Expr<T>>,
        source: Source,
        type_: T,
    },
    AccessStruct {
        // left.right
        left: Box<Expr<T>>,
        right: Identifier,
        type_: T,
    },
    CreateArray {
        fields: Vec<Expr<T>>,
        source: Source,
        type_: Typed,
    },
    CreateStruct {
        fields: Vec<(Identifier, Expr<T>)>,
        source: Source,
        type_: Typed,
    },
    If {
        condition: Box<Expr<T>>,
        if_branch: Box<Stmt<T>>,
        else_branch: Option<Box<Stmt<T>>>,
        source: Source,
        type_: T,
    },
    Literal {
        literal: ExprOrigin<Literal>,
        type_: T,
    },
    Variable {
        name: Identifier,
        type_: T,
    },
}

pub trait UntypedExprVisitor<'a> {
    type Output;

    fn visit_expr(&mut self, expr: &'a UntypedExpr) -> Self::Output {
        match expr.kind() {
            ExprKind::Unary {
                operator,
                right,
                type_: _,
            } => self.visit_unary(operator, right),
            ExprKind::Binary {
                left,
                operator,
                right,
                type_: _,
            } => self.visit_binary(left, operator, right),
            ExprKind::Call {
                callee,
                arguments,
                source,
                type_: _,
            } => self.visit_call(callee, arguments, source),
            ExprKind::AccessStruct {
                left,
                right,
                type_: _,
            } => self.visit_access_struct(left, right),
            ExprKind::IndexArray {
                object,
                index,
                source,
                type_: _,
            } => self.visit_index_array(object, index, source),
            ExprKind::CreateArray {
                fields,
                source,
                type_: _,
            } => self.visit_create_array(fields, source),
            ExprKind::CreateStruct {
                fields,
                source,
                type_: _,
            } => self.visit_create_struct(fields, source),
            ExprKind::If {
                condition,
                if_branch,
                else_branch,
                source,
                type_: _,
            } => self.visit_if(condition, if_branch, else_branch, source),
            ExprKind::Literal { literal, type_: _ } => self.visit_literal(literal),
            ExprKind::Variable { name, type_: _ } => self.visit_variable(name),
        }
    }

    fn visit_unary(
        &mut self,
        operator: &'a ExprOrigin<UnaryOp>,
        right: &'a UntypedExpr,
    ) -> Self::Output;

    fn visit_binary(
        &mut self,
        left: &'a UntypedExpr,
        operator: &'a ExprOrigin<BinaryOp>,
        right: &'a UntypedExpr,
    ) -> Self::Output;

    fn visit_call(
        &mut self,
        callee: &'a UntypedExpr,
        arguments: &'a [UntypedExpr],
        source: &'a Source,
    ) -> Self::Output;

    fn visit_access_struct(&mut self, left: &'a UntypedExpr, right: &'a Identifier)
    -> Self::Output;

    fn visit_index_array(
        &mut self,
        object: &'a UntypedExpr,
        index: &'a UntypedExpr,
        source: &'a Source,
    ) -> Self::Output;

    fn visit_create_array(&mut self, fields: &'a [UntypedExpr], source: &'a Source)
    -> Self::Output;

    fn visit_create_struct(
        &mut self,
        fields: &'a [(Identifier, UntypedExpr)],
        source: &'a Source,
    ) -> Self::Output;

    fn visit_if(
        &mut self,
        condition: &'a UntypedExpr,
        if_branch: &'a Box<UntypedStmt>,
        else_branch: &'a Option<Box<UntypedStmt>>,
        source: &'a Source,
    ) -> Self::Output;

    fn visit_literal(&mut self, value: &'a ExprOrigin<Literal>) -> Self::Output;

    fn visit_variable(&mut self, name: &'a Identifier) -> Self::Output;
}

pub trait TypedExprVisitor<'a> {
    type Type;
    type Output;

    fn visit_expr(&mut self, expr: &'a Expr<Self::Type>) -> Self::Output {
        match expr.kind() {
            ExprKind::Unary {
                operator,
                right,
                type_,
            } => self.visit_unary(operator, right, type_),
            ExprKind::Binary {
                left,
                operator,
                right,
                type_,
            } => self.visit_binary(left, operator, right, type_),
            ExprKind::AccessStruct { left, right, type_ } => self.visit_get(left, right, type_),
            ExprKind::IndexArray {
                object,
                index,
                source,
                type_,
            } => self.visit_index(object, index, source, type_),
            ExprKind::Call {
                callee,
                arguments,
                source,
                type_,
            } => self.visit_call(callee, arguments, source, type_),
            ExprKind::CreateArray {
                fields,
                source,
                type_,
            } => self.visit_create_array(fields, source, type_),
            ExprKind::CreateStruct {
                fields,
                source,
                type_,
            } => self.visit_create_struct(fields, source, type_),
            ExprKind::If {
                condition,
                if_branch,
                else_branch,
                source,
                type_,
            } => self.visit_if(condition, if_branch, else_branch, source, type_),
            ExprKind::Literal { literal, type_ } => self.visit_literal(literal, type_),
            ExprKind::Variable { name, type_ } => self.visit_variable(name, type_),
        }
    }

    fn visit_unary(
        &mut self,
        operator: &'a ExprOrigin<UnaryOp>,
        right: &'a Expr<Self::Type>,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_binary(
        &mut self,
        left: &'a Expr<Self::Type>,
        operator: &'a ExprOrigin<BinaryOp>,
        right: &'a Expr<Self::Type>,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_get(
        &mut self,
        left: &'a Expr<Self::Type>,
        right: &'a Identifier,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_index(
        &mut self,
        object: &'a Expr<Self::Type>,
        index: &'a Expr<Self::Type>,
        source: &'a Source,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_call(
        &mut self,
        callee: &'a Expr<Self::Type>,
        arguments: &'a [Expr<Self::Type>],
        source: &'a Source,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_create_array(
        &mut self,
        fields: &'a [Expr<Self::Type>],
        source: &'a Source,
        type_: &'a Typed,
    ) -> Self::Output;

    fn visit_create_struct(
        &mut self,
        fields: &'a [(Identifier, Expr<Self::Type>)],
        source: &'a Source,
        type_: &'a Typed,
    ) -> Self::Output;

    fn visit_if(
        &mut self,
        condition: &'a Expr<Self::Type>,
        if_branch: &'a Box<Stmt<Self::Type>>,
        else_branch: &'a Option<Box<Stmt<Self::Type>>>,
        source: &'a Source,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_literal(
        &mut self,
        value: &'a ExprOrigin<Literal>,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_variable(&mut self, name: &'a Identifier, type_: &'a Self::Type) -> Self::Output;
}

pub trait ExprFolder {
    type Type;
    type Output;

    fn fold_expr(&mut self, expr: Expr<Self::Type>) -> Self::Output {
        match expr.kind {
            ExprKind::Unary {
                operator,
                right,
                type_,
            } => self.fold_unary(operator, *right, type_),
            ExprKind::Binary {
                left,
                operator,
                right,
                type_,
            } => self.fold_binary(*left, operator, *right, type_),
            ExprKind::AccessStruct { left, right, type_ } => self.fold_get(*left, right, type_),
            ExprKind::IndexArray {
                object,
                index,
                source,
                type_,
            } => self.fold_index(*object, *index, source, type_),
            ExprKind::Call {
                callee,
                arguments,
                source,
                type_,
            } => self.fold_call(*callee, arguments, source, type_),
            ExprKind::CreateArray {
                fields,
                source,
                type_,
            } => self.fold_create_array(fields, source, type_),
            ExprKind::CreateStruct {
                fields,
                source,
                type_,
            } => self.fold_create_struct(fields, source, type_),
            ExprKind::If {
                condition,
                if_branch,
                else_branch,
                source,
                type_,
            } => self.fold_if(*condition, if_branch, else_branch, source, type_),
            ExprKind::Literal { literal, type_ } => self.fold_literal(literal, type_),
            ExprKind::Variable { name, type_ } => self.fold_variable(name, type_),
        }
    }

    fn fold_unary(
        &mut self,
        operator: ExprOrigin<UnaryOp>,
        right: Expr<Self::Type>,
        type_: Self::Type,
    ) -> Self::Output;

    fn fold_binary(
        &mut self,
        left: Expr<Self::Type>,
        operator: ExprOrigin<BinaryOp>,
        right: Expr<Self::Type>,
        type_: Self::Type,
    ) -> Self::Output;

    fn fold_get(
        &mut self,
        left: Expr<Self::Type>,
        right: Identifier,
        type_: Self::Type,
    ) -> Self::Output;

    fn fold_index(
        &mut self,
        object: Expr<Self::Type>,
        index: Expr<Self::Type>,
        source: Source,
        type_: Self::Type,
    ) -> Self::Output;

    fn fold_call(
        &mut self,
        callee: Expr<Self::Type>,
        arguments: Vec<Expr<Self::Type>>,
        source: Source,
        type_: Self::Type,
    ) -> Self::Output;

    fn fold_create_array(
        &mut self,
        fields: Vec<Expr<Self::Type>>,
        source: Source,
        type_: Typed,
    ) -> Self::Output;

    fn fold_create_struct(
        &mut self,
        fields: Vec<(Identifier, Expr<Self::Type>)>,
        source: Source,
        type_: Typed,
    ) -> Self::Output;

    fn fold_if(
        &mut self,
        condition: Expr<Self::Type>,
        if_branch: Box<Stmt<Self::Type>>,
        else_branch: Option<Box<Stmt<Self::Type>>>,
        source: Source,
        type_: Self::Type,
    ) -> Self::Output;

    fn fold_literal(&mut self, value: ExprOrigin<Literal>, type_: Self::Type) -> Self::Output;

    fn fold_variable(&mut self, name: Identifier, type_: Self::Type) -> Self::Output;
}
