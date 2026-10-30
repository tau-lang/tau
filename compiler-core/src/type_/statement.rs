use crate::{
    analyser::SemanticAnalyser,
    ast::{
        TypedStmt,
        expression::Expr,
        identifier::Identifier,
        prelude::*,
        statement::{Stmt, StmtFolder},
    },
    error::{Error, Result, label, primary},
    lexer::Source,
    type_::{Typed, Untyped, prelude::*, unify},
};

impl StmtFolder for SemanticAnalyser {
    type Type = Untyped;
    type Output = Result<TypedStmt>;

    fn fold_block(&mut self, statements: Vec<Stmt<Self::Type>>, source: Source) -> Self::Output {
        let statements = self.fold_stmts(statements)?;
        Ok(block(statements, source))
    }

    fn fold_let(
        &mut self,
        name: Identifier,
        declared: Option<Typed>,
        initializer: Expr<Self::Type>,
        source: Source,
    ) -> Self::Output {
        let initializer = self.infer(initializer)?;
        let type_ = if let Some(declared) = declared {
            unify(initializer.type_().clone(), declared.clone()).map_err(|d| {
                Error::with_diagnostic(
                    d.with_label(primary(source))
                        .with_label(label(format!("declared type was {}", declared), &name))
                        .with_label(label(
                            format!("initializer has type {}", initializer.type_()),
                            &initializer,
                        )),
                )
            })?;
            declared
        } else {
            initializer.type_().clone()
        };
        self.environment.declare_var(&name, type_.clone())?;
        Ok(let_(name, Some(type_), initializer, source))
    }

    fn fold_return(&mut self, value: Expr<Self::Type>, source: Source) -> Self::Output {
        let value = self.infer(value)?;
        Ok(return_(value, source))
    }

    fn fold_break(&mut self, source: Source) -> Self::Output {
        Ok(break_(source))
    }

    fn fold_while(
        &mut self,
        condition: Expr<Self::Type>,
        body: Box<Stmt<Self::Type>>,
        source: Source,
    ) -> Self::Output {
        let condition = self.infer_and_unify(condition, bool())?;
        let body = self.fold_stmt(*body)?;
        Ok(while_(condition, body, source))
    }

    fn fold_expr_stmt(&mut self, expr: Expr<Self::Type>, source: Source) -> Self::Output {
        let expr = self.infer(expr)?;
        Ok(expr_stmt(expr, source))
    }
}
