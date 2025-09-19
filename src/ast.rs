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
    Get {
        left: Rc<Expr>,
        right: Token,
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
    CreateArray {
        array_type: Token,
        array_size: Option<Rc<Expr>>,
        fields: Vec<Rc<Expr>>,
    },
    CreateStruct {
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

pub trait ExprVisitor<'a, T> {
    fn visit_expr(&mut self, expr: &'a Expr) -> T {
        match expr {
            Expr::Unary { operator, right } => self.visit_unary(operator, right),
            Expr::Binary {
                left,
                operator,
                right,
            } => self.visit_binary(left, operator, right),
            Expr::Get { left, right } => self.visit_get(left, right),
            Expr::Index { object, index } => self.visit_index(object, index),
            Expr::Call { callee, arguments } => self.visit_call(callee, arguments),
            Expr::CreateArray {
                array_type,
                array_size,
                fields,
            } => self.visit_create_array(array_type, array_size, fields),
            Expr::CreateStruct {
                struct_name,
                fields,
            } => self.visit_create_struct(struct_name, fields),
            Expr::If {
                condition,
                if_branch,
                else_branch,
            } => self.visit_if(condition, if_branch, else_branch),
            Expr::Literal(value) => self.visit_literal(value),
            Expr::Variable(name) => self.visit_variable(name),
        }
    }

    fn visit_unary(&mut self, operator: &'a Token, right: &'a Rc<Expr>) -> T;

    fn visit_binary(&mut self, left: &'a Rc<Expr>, operator: &'a Token, right: &'a Rc<Expr>) -> T;

    fn visit_get(&mut self, left: &'a Rc<Expr>, right: &'a Token) -> T;

    fn visit_index(&mut self, object: &'a Rc<Expr>, index: &'a Rc<Expr>) -> T;

    fn visit_call(&mut self, callee: &'a Rc<Expr>, arguments: &'a [Rc<Expr>]) -> T;

    fn visit_create_array(
        &mut self,
        array_type: &'a Token,
        array_size: &'a Option<Rc<Expr>>,
        fields: &'a [Rc<Expr>],
    ) -> T;

    fn visit_create_struct(&mut self, struct_name: &'a Token, fields: &'a [(Token, Rc<Expr>)])
    -> T;

    fn visit_if(
        &mut self,
        condition: &'a Rc<Expr>,
        if_branch: &'a Rc<Stmt>,
        else_branch: &'a Option<Rc<Stmt>>,
    ) -> T;

    fn visit_literal(&mut self, value: &'a Token) -> T;

    fn visit_variable(&mut self, name: &'a Token) -> T;
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum Stmt {
    Block {
        statements: Vec<Rc<Stmt>>,
    },
    Let {
        name: Token,
        var_type: Option<Token>,
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
        name: &'a Token,
        var_type: &'a Option<Token>,
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
        name: Token,
        fields: Vec<(Token, Token)>,
        methods: Vec<Rc<Decl>>,
    },
    Function {
        name: Token,
        return_type: Option<Token>,
        params: Vec<(Token, Token)>,
        body: Vec<Stmt>,
    },
    Const {
        name: Token,
        var_type: Token,
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
            } => self.visit_function(name, return_type, params, body),
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
        name: &'a Token,
        fields: &'a [(Token, Token)],
        methods: &'a [Rc<Decl>],
    ) -> T;

    fn visit_function(
        &mut self,
        name: &'a Token,
        return_type: &'a Option<Token>,
        params: &'a [(Token, Token)],
        body: &'a [Stmt],
    ) -> T;

    fn visit_const(&mut self, name: &'a Token, var_type: &'a Token, initializer: &'a Expr) -> T;
}
