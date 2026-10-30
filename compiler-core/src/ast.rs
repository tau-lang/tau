use crate::{
    lexer::Source,
    type_::{
        Type,
        prelude::{Typed, Untyped},
    },
};
use std::sync::Arc;

pub mod declaration;
pub mod expression;
pub mod identifier;
#[macro_use]
pub mod macros;
pub mod origin;
pub mod prelude;
pub mod statement;

pub use declaration::Decl;
pub use expression::Expr;
pub use identifier::Identifier;
pub use statement::Stmt;

pub trait Location {
    fn source(&self) -> Source;

    fn merge(&self, location: &impl Location) -> Source {
        self.source().merge(location.source())
    }
}

impl<L> From<&L> for Source
where
    L: Location,
{
    fn from(value: &L) -> Source {
        value.source()
    }
}

pub type UntypedExpr = Expr<Untyped>;
pub type UntypedStmt = Stmt<Untyped>;
pub type UntypedDecl = Decl<Untyped>;

pub type TypedExpr = Expr<Arc<Type>>;
pub type TypedStmt = Stmt<Arc<Type>>;
pub type TypedDecl = Decl<Arc<Type>>;
