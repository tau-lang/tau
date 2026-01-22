use crate::{
    ast::{identifier::Identifier, statement::Stmt},
    lexer::{Source, Token},
    typing::TypeCell,
};
use std::rc::Rc;

#[derive(Debug)]
pub struct Expr {
    kind: ExprKind,
    source: Source,
}

#[derive(Debug)]
pub enum ExprKind {
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
    Get {
        left: Rc<Expr>,
        right: Identifier,
        lookup: TypeCell,
    },
    Index {
        // object[index]
        object: Rc<Expr>,
        index: Rc<Expr>,
        lookup: TypeCell,
    },
    Call {
        // callee(..arguments)
        callee: Rc<Expr>,
        arguments: Vec<Rc<Expr>>,
    },
    CreateArray {
        array_type: TypeCell,
        array_size: Option<Rc<Expr>>,
        fields: Vec<Rc<Expr>>,
    },
    CreateStruct {
        struct_type: TypeCell,
        fields: Vec<(Identifier, Rc<Expr>)>,
    },
    If {
        condition: Rc<Expr>,
        if_branch: Rc<Stmt>,
        else_branch: Option<Rc<Stmt>>,
        expression_type: TypeCell,
    },
    Literal(Token),
    Variable {
        name: Identifier,
        variable_type: TypeCell,
    },
}

impl Expr {
    pub fn new(source: Source, kind: ExprKind) -> Self {
        Self { kind, source }
    }
    pub fn source(&self) -> Source {
        self.source.clone()
    }
    pub fn kind(&self) -> &ExprKind {
        &self.kind
    }
}

pub trait ExprVisitor<'a, T> {
    fn visit_expr(&mut self, expr: &'a Expr) -> T {
        match expr.kind() {
            ExprKind::Unary { operator, right } => self.visit_unary(operator, right),
            ExprKind::Binary {
                left,
                operator,
                right,
            } => self.visit_binary(left, operator, right),
            ExprKind::Get {
                left,
                right,
                lookup,
            } => self.visit_get(left, right, lookup),
            ExprKind::Index {
                object,
                index,
                lookup,
            } => self.visit_index(object, index, lookup),
            ExprKind::Call { callee, arguments } => self.visit_call(callee, arguments),
            ExprKind::CreateArray {
                array_type,
                array_size,
                fields,
            } => self.visit_create_array(array_type, array_size, fields),
            ExprKind::CreateStruct {
                struct_type,
                fields,
            } => self.visit_create_struct(struct_type, fields),
            ExprKind::If {
                condition,
                if_branch,
                else_branch,
                expression_type,
            } => self.visit_if(condition, if_branch, else_branch, expression_type),
            ExprKind::Literal(value) => self.visit_literal(value),
            ExprKind::Variable {
                name,
                variable_type,
            } => self.visit_variable(name, variable_type),
        }
    }

    fn visit_unary(&mut self, operator: &'a Token, right: &'a Rc<Expr>) -> T;

    fn visit_binary(&mut self, left: &'a Rc<Expr>, operator: &'a Token, right: &'a Rc<Expr>) -> T;

    fn visit_get(&mut self, left: &'a Rc<Expr>, right: &'a Identifier, lookup: &'a TypeCell) -> T;

    fn visit_index(&mut self, object: &'a Rc<Expr>, index: &'a Rc<Expr>, lookup: &'a TypeCell)
    -> T;

    fn visit_call(&mut self, callee: &'a Rc<Expr>, arguments: &'a [Rc<Expr>]) -> T;

    fn visit_create_array(
        &mut self,
        array_type: &'a TypeCell,
        array_size: &'a Option<Rc<Expr>>,
        fields: &'a [Rc<Expr>],
    ) -> T;

    fn visit_create_struct(
        &mut self,
        struct_name: &'a TypeCell,
        fields: &'a [(Identifier, Rc<Expr>)],
    ) -> T;

    fn visit_if(
        &mut self,
        condition: &'a Rc<Expr>,
        if_branch: &'a Rc<Stmt>,
        else_branch: &'a Option<Rc<Stmt>>,
        expression_result: &'a TypeCell,
    ) -> T;

    fn visit_literal(&mut self, value: &'a Token) -> T;

    fn visit_variable(&mut self, name: &'a Identifier, var_type: &'a TypeCell) -> T;
}
