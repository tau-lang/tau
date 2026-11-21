use crate::{
    ast::{
        Location, Unfold, UntypedExpr,
        identifier::Identifier,
        lookup::{CallLookup, VarLookup},
        origin::{BinaryOp, Literal, Origin, UnaryOp},
    },
    lexer::Source,
    type_::{Typed, Untyped},
};
use std::boxed::Box;

#[derive(PartialEq, Debug)]
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

impl<T> Unfold for Expr<T> {
    type Unfolded = ExprKind<T>;

    fn unfold(self) -> Self::Unfolded {
        self.kind
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
            | ExprKind::Ternary { type_, .. }
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
            | ExprKind::Ternary { source, .. } => *source,
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
            ExprKind::Variable { lookup, .. } => lookup.source(),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum ExprKind<T> {
    Unary {
        // -right
        operator: Origin<UnaryOp>,
        right: Box<Expr<T>>,
        type_: T,
    },
    Binary {
        // left + right
        left: Box<Expr<T>>,
        operator: Origin<BinaryOp>,
        right: Box<Expr<T>>,
        type_: T,
    },
    Call {
        // lookup(..arguments)
        lookup: CallLookup<T>,
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
    Ternary {
        condition: Box<Expr<T>>,
        if_branch: Box<Expr<T>>,
        else_branch: Box<Expr<T>>,
        source: Source,
        type_: T,
    },
    Literal {
        literal: Origin<Literal>,
        type_: T,
    },
    Variable {
        lookup: VarLookup,
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
                lookup,
                arguments,
                source,
                type_: _,
            } => self.visit_call(lookup, arguments, source),
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
            ExprKind::Ternary {
                condition,
                if_branch,
                else_branch,
                source,
                type_: _,
            } => self.visit_ternary(condition, if_branch, else_branch, source),
            ExprKind::Literal { literal, type_: _ } => self.visit_literal(literal),
            ExprKind::Variable { lookup, type_: _ } => self.visit_variable(lookup),
        }
    }

    fn visit_unary(
        &mut self,
        operator: &'a Origin<UnaryOp>,
        right: &'a UntypedExpr,
    ) -> Self::Output;

    fn visit_binary(
        &mut self,
        left: &'a UntypedExpr,
        operator: &'a Origin<BinaryOp>,
        right: &'a UntypedExpr,
    ) -> Self::Output;

    fn visit_call(
        &mut self,
        lookup: &'a CallLookup<Untyped>,
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

    fn visit_ternary(
        &mut self,
        condition: &'a UntypedExpr,
        if_branch: &'a UntypedExpr,
        else_branch: &'a UntypedExpr,
        source: &'a Source,
    ) -> Self::Output;

    fn visit_literal(&mut self, value: &'a Origin<Literal>) -> Self::Output;

    fn visit_variable(&mut self, lookup: &'a VarLookup) -> Self::Output;
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
            ExprKind::AccessStruct { left, right, type_ } => {
                self.visit_access_struct(left, right, type_)
            }
            ExprKind::IndexArray {
                object,
                index,
                source,
                type_,
            } => self.visit_index_array(object, index, source, type_),
            ExprKind::Call {
                lookup,
                arguments,
                source,
                type_,
            } => self.visit_call(lookup, arguments, source, type_),
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
            ExprKind::Ternary {
                condition,
                if_branch,
                else_branch,
                source,
                type_,
            } => self.visit_ternary(condition, if_branch, else_branch, source, type_),
            ExprKind::Literal { literal, type_ } => self.visit_literal(literal, type_),
            ExprKind::Variable { lookup, type_ } => self.visit_variable(lookup, type_),
        }
    }

    fn visit_unary(
        &mut self,
        operator: &'a Origin<UnaryOp>,
        right: &'a Expr<Self::Type>,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_binary(
        &mut self,
        left: &'a Expr<Self::Type>,
        operator: &'a Origin<BinaryOp>,
        right: &'a Expr<Self::Type>,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_access_struct(
        &mut self,
        left: &'a Expr<Self::Type>,
        right: &'a Identifier,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_index_array(
        &mut self,
        object: &'a Expr<Self::Type>,
        index: &'a Expr<Self::Type>,
        source: &'a Source,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_call(
        &mut self,
        lookup: &'a CallLookup<Self::Type>,
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

    fn visit_ternary(
        &mut self,
        condition: &'a Expr<Self::Type>,
        if_branch: &'a Expr<Self::Type>,
        else_branch: &'a Expr<Self::Type>,
        source: &'a Source,
        type_: &'a Self::Type,
    ) -> Self::Output;

    fn visit_literal(&mut self, value: &'a Origin<Literal>, type_: &'a Self::Type) -> Self::Output;

    fn visit_variable(&mut self, lookup: &'a VarLookup, type_: &'a Self::Type) -> Self::Output;
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
            ExprKind::AccessStruct { left, right, type_ } => {
                self.fold_access_struct(*left, right, type_)
            }
            ExprKind::IndexArray {
                object,
                index,
                source,
                type_,
            } => self.fold_index_array(*object, *index, source, type_),
            ExprKind::Call {
                lookup,
                arguments,
                source,
                type_,
            } => self.fold_call(lookup, arguments, source, type_),
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
            ExprKind::Ternary {
                condition,
                if_branch,
                else_branch,
                source,
                type_,
            } => self.fold_ternary(*condition, *if_branch, *else_branch, source, type_),
            ExprKind::Literal { literal, type_ } => self.fold_literal(literal, type_),
            ExprKind::Variable { lookup, type_ } => self.fold_variable(lookup, type_),
        }
    }

    fn fold_unary(
        &mut self,
        operator: Origin<UnaryOp>,
        right: Expr<Self::Type>,
        type_: Self::Type,
    ) -> Self::Output;

    fn fold_binary(
        &mut self,
        left: Expr<Self::Type>,
        operator: Origin<BinaryOp>,
        right: Expr<Self::Type>,
        type_: Self::Type,
    ) -> Self::Output;

    fn fold_access_struct(
        &mut self,
        left: Expr<Self::Type>,
        right: Identifier,
        type_: Self::Type,
    ) -> Self::Output;

    fn fold_index_array(
        &mut self,
        object: Expr<Self::Type>,
        index: Expr<Self::Type>,
        source: Source,
        type_: Self::Type,
    ) -> Self::Output;

    fn fold_call(
        &mut self,
        lookup: CallLookup<Self::Type>,
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

    fn fold_ternary(
        &mut self,
        condition: Expr<Self::Type>,
        if_branch: Expr<Self::Type>,
        else_branch: Expr<Self::Type>,
        source: Source,
        type_: Self::Type,
    ) -> Self::Output;

    fn fold_literal(&mut self, value: Origin<Literal>, type_: Self::Type) -> Self::Output;

    fn fold_variable(&mut self, lookup: VarLookup, type_: Self::Type) -> Self::Output;
}
