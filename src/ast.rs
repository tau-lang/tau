use crate::{
    lexer::{Source, Token},
    typing::TypeCell,
};
use std::{
    fmt::{Display, Formatter, Result},
    rc::Rc,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    name: String,
    source: Source,
}

impl Identifier {
    pub fn new(name: String, source: Source) -> Self {
        Identifier { name, source }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

impl Display for Identifier {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        formatter.write_str(&self.name)
    }
}

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
    Get {
        left: Rc<Expr>,
        right: Token,
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

pub trait ExprVisitor<'a, T> {
    fn visit_expr(&mut self, expr: &'a Expr) -> T {
        match expr {
            Expr::Unary { operator, right } => self.visit_unary(operator, right),
            Expr::Binary {
                left,
                operator,
                right,
            } => self.visit_binary(left, operator, right),
            Expr::Get {
                left,
                right,
                lookup,
            } => self.visit_get(left, right, lookup),
            Expr::Index {
                object,
                index,
                lookup,
            } => self.visit_index(object, index, lookup),
            Expr::Call { callee, arguments } => self.visit_call(callee, arguments),
            Expr::CreateArray {
                array_type,
                array_size,
                fields,
            } => self.visit_create_array(array_type, array_size, fields),
            Expr::CreateStruct {
                struct_type,
                fields,
            } => self.visit_create_struct(struct_type, fields),
            Expr::If {
                condition,
                if_branch,
                else_branch,
                expression_type,
            } => self.visit_if(condition, if_branch, else_branch, expression_type),
            Expr::Literal(value) => self.visit_literal(value),
            Expr::Variable {
                name,
                variable_type,
            } => self.visit_variable(name, variable_type),
        }
    }

    fn visit_unary(&mut self, operator: &'a Token, right: &'a Rc<Expr>) -> T;

    fn visit_binary(&mut self, left: &'a Rc<Expr>, operator: &'a Token, right: &'a Rc<Expr>) -> T;

    fn visit_get(&mut self, left: &'a Rc<Expr>, right: &'a Token, lookup: &'a TypeCell) -> T;

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

#[derive(Debug)]
#[allow(dead_code)]
pub enum Stmt {
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
        match stmt {
            Stmt::Block { statements } => self.visit_block(statements),
            Stmt::Let {
                name,
                var_type,
                initializer,
            } => self.visit_let(name, var_type, initializer),
            Stmt::Return { value } => self.visit_return(value),
            Stmt::Break => self.visit_break(),
            Stmt::While { condition, body } => self.visit_while(condition, body),
            Stmt::For {
                initializer,
                condition,
                increment,
                body,
            } => self.visit_for(initializer, condition, increment, body),
            Stmt::ExprStmt(expr) => self.visit_expr_stmt(expr),
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

#[derive(Debug)]
pub enum Decl {
    Import(Token),
    Struct {
        name: Identifier,
        fields: Vec<(Identifier, TypeCell)>,
        methods: Vec<Rc<Decl>>,
    },
    Function {
        name: Identifier,
        return_type: TypeCell,
        params: Vec<(Identifier, TypeCell)>,
        body: Vec<Stmt>,
        is_extern: bool,
    },
    Const {
        name: Identifier,
        var_type: TypeCell,
        initializer: Expr,
    },
}

pub trait DeclVisitor<'a, T> {
    fn visit_decl(&mut self, decl: &'a Decl) -> T {
        match decl {
            Decl::Import(name) => self.visit_import(name),
            Decl::Struct {
                name,
                fields,
                methods,
            } => self.visit_struct(name, fields, methods),
            Decl::Function {
                name,
                return_type,
                params,
                body,
                is_extern,
            } => self.visit_function(name, return_type, params, body, *is_extern),
            Decl::Const {
                name,
                var_type,
                initializer,
            } => self.visit_const(name, var_type, initializer),
        }
    }

    fn visit_import(&mut self, name: &'a Token) -> T;

    fn visit_struct(
        &mut self,
        name: &'a Identifier,
        fields: &'a [(Identifier, TypeCell)],
        methods: &'a [Rc<Decl>],
    ) -> T;

    fn visit_function(
        &mut self,
        name: &'a Identifier,
        return_type: &'a TypeCell,
        params: &'a [(Identifier, TypeCell)],
        body: &'a [Stmt],
        is_extern: bool,
    ) -> T;

    fn visit_const(
        &mut self,
        name: &'a Identifier,
        var_type: &'a TypeCell,
        initializer: &'a Expr,
    ) -> T;
}
