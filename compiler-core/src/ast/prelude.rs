use crate::{
    ast::{
        declaration::{Decl, DeclFunc, DeclKind, DeclStruct},
        expression::{Expr, ExprKind},
        identifier::Identifier,
        origin::{BinaryOp, ExprOrigin, Literal, UnaryOp},
        statement::{Stmt, StmtKind},
    },
    lexer::Source,
    type_::Typed,
};

pub fn import<T>(path: Vec<Identifier>, source: Source) -> Decl<T> {
    Decl::new(DeclKind::Import(path), source)
}

pub fn struct_<T>(struct_: DeclStruct, source: Source) -> Decl<T> {
    Decl::new(DeclKind::Struct(struct_), source)
}

pub fn func<T>(func: DeclFunc<T>, source: Source) -> Decl<T> {
    Decl::new(DeclKind::Func(func), source)
}

pub fn const_<T>(name: Identifier, type_: Typed, initializer: Expr<T>, source: Source) -> Decl<T> {
    Decl::new(
        DeclKind::Const {
            name,
            type_,
            initializer,
        },
        source,
    )
}

pub fn block<T>(statements: Vec<Stmt<T>>, source: Source) -> Stmt<T> {
    Stmt::new(StmtKind::Block { statements }, source)
}

pub fn let_<T>(
    name: Identifier,
    declared: Option<Typed>,
    initializer: Expr<T>,
    source: Source,
) -> Stmt<T> {
    Stmt::new(
        StmtKind::Let {
            name,
            declared,
            initializer,
        },
        source,
    )
}

pub fn return_<T>(value: Expr<T>, source: Source) -> Stmt<T> {
    Stmt::new(StmtKind::Return { value }, source)
}

pub fn break_<T>(source: Source) -> Stmt<T> {
    Stmt::new(StmtKind::Break, source)
}

pub fn while_<T>(condition: Expr<T>, body: Stmt<T>, source: Source) -> Stmt<T> {
    Stmt::new(
        StmtKind::While {
            condition,
            body: Box::new(body),
        },
        source,
    )
}

pub fn expr_stmt<T>(expr: Expr<T>, source: Source) -> Stmt<T> {
    Stmt::new(StmtKind::ExprStmt(expr), source)
}

pub fn unary<T>(operator: ExprOrigin<UnaryOp>, right: Expr<T>, type_: T) -> Expr<T> {
    Expr::new(ExprKind::Unary {
        operator,
        right: Box::new(right),
        type_,
    })
}

pub fn binary<T>(
    left: Expr<T>,
    operator: ExprOrigin<BinaryOp>,
    right: Expr<T>,
    type_: T,
) -> Expr<T> {
    Expr::new(ExprKind::Binary {
        left: Box::new(left),
        operator,
        right: Box::new(right),
        type_,
    })
}

pub fn call<T>(callee: Expr<T>, arguments: Vec<Expr<T>>, source: Source, type_: T) -> Expr<T> {
    Expr::new(ExprKind::Call {
        callee: Box::new(callee),
        arguments,
        source,
        type_,
    })
}

pub fn index_array<T>(object: Expr<T>, index: Expr<T>, source: Source, type_: T) -> Expr<T> {
    Expr::new(ExprKind::IndexArray {
        object: Box::new(object),
        index: Box::new(index),
        source,
        type_,
    })
}

pub fn access_struct<T>(left: Expr<T>, right: Identifier, type_: T) -> Expr<T> {
    Expr::new(ExprKind::AccessStruct {
        left: Box::new(left),
        right,
        type_,
    })
}

pub fn create_array<T>(fields: Vec<Expr<T>>, source: Source, type_: Typed) -> Expr<T> {
    Expr::new(ExprKind::CreateArray {
        fields,
        source,
        type_,
    })
}

pub fn create_struct<T>(
    fields: Vec<(Identifier, Expr<T>)>,
    source: Source,
    type_: Typed,
) -> Expr<T> {
    Expr::new(ExprKind::CreateStruct {
        fields,
        source,
        type_,
    })
}

pub fn if_<T>(
    condition: Expr<T>,
    if_branch: Stmt<T>,
    else_branch: Option<Stmt<T>>,
    source: Source,
    type_: T,
) -> Expr<T> {
    Expr::new(ExprKind::If {
        condition: Box::new(condition),
        if_branch: Box::new(if_branch),
        else_branch: else_branch.map(|b| Box::new(b)),
        source,
        type_,
    })
}

pub fn literal<T>(literal: ExprOrigin<Literal>, type_: T) -> Expr<T> {
    Expr::new(ExprKind::Literal { literal, type_ })
}

pub fn variable<T>(name: Identifier, type_: T) -> Expr<T> {
    Expr::new(ExprKind::Variable { name, type_ })
}
