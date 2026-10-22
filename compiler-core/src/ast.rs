use crate::{lexer::Source, type_::Type};
use std::sync::Arc;

pub mod declaration;
pub mod expression;
pub mod identifier;
pub mod statement;

pub use declaration::Decl;
pub use expression::Expr;
pub use identifier::Identifier;
pub use statement::Stmt;

pub trait Location {
    fn source(&self) -> &Source;
}

pub type Untyped = ();
pub type Typed = Arc<Type>;

pub type UntypedExpr = Expr<Untyped>;
pub type UntypedStmt = Stmt<Untyped>;
pub type UntypedDecl = Decl<Untyped>;

pub type TypedExpr = Expr<Arc<Type>>;
pub type TypedStmt = Stmt<Arc<Type>>;
pub type TypedDecl = Expr<Arc<Type>>;
