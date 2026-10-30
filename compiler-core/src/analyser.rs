use crate::{
    ast::{
        TypedDecl, TypedExpr, TypedStmt, UntypedDecl, UntypedExpr, UntypedStmt,
        declaration::DeclFolder, expression::ExprFolder, statement::StmtFolder,
    },
    error::{Error, Result, label, primary},
    fold_vec,
    type_::{Typed, environment::Environment, unify},
};

pub struct SemanticAnalyser {
    pub(crate) environment: Environment,
}

impl SemanticAnalyser {
    pub fn new(environment: Environment) -> Self {
        Self { environment }
    }

    pub fn env(self) -> Environment {
        self.environment
    }

    pub fn new_unbound_var(&mut self) -> Typed {
        self.environment.new_unbound_var()
    }

    pub fn infer(&mut self, expr: UntypedExpr) -> Result<TypedExpr> {
        self.fold_expr(expr)
    }

    /// Infers the type of the given function and tries to unify it with the
    /// given type, recording any unification error that might take place.
    pub fn infer_and_unify(&mut self, expression: UntypedExpr, type_: Typed) -> Result<TypedExpr> {
        let expression = self.infer(expression)?;
        unify(type_, expression.type_().clone()).map_err(|d| {
            Error::with_diagnostic(d.with_label(primary(&expression)).with_label(label(
                format!("expression has type {}", expression.type_()),
                &expression,
            )))
        })?;
        Ok(expression)
    }

    pub fn fold_ast(&mut self, ast: Vec<UntypedDecl>) -> Result<Vec<TypedDecl>> {
        fold_vec!(self, ast, fold_decl)
    }

    pub fn fold_stmts(&mut self, statements: Vec<UntypedStmt>) -> Result<Vec<TypedStmt>> {
        fold_vec!(self, statements, fold_stmt)
    }
}
