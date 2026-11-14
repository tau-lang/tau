use crate::{
    analyzer::SemanticAnalyzer,
    ast::{
        TypedStmt,
        expression::{Expr, TypedExprVisitor},
        fold_vec,
        identifier::Identifier,
        prelude::*,
        statement::{Stmt, StmtFolder, StmtVisitor},
    },
    error::{Error, Result, label, primary},
    lexer::Source,
    type_::{Typed, Untyped, prelude::*, unify},
};

impl<'a> StmtFolder for SemanticAnalyzer<'a> {
    type Type = Untyped;
    type Output = Result<TypedStmt>;

    fn fold_block(&mut self, statements: Vec<Stmt<Self::Type>>, source: Source) -> Self::Output {
        let statements = fold_vec(statements, |stmt| self.fold_stmt(stmt))?;
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
                Error::new(
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

    fn fold_return(&mut self, value: Option<Expr<Self::Type>>, source: Source) -> Self::Output {
        let value = if let Some(value) = value {
            Some(self.infer_and_unify(value, self.environment.return_type().clone())?)
        } else {
            None
        };
        Ok(return_(value, source))
    }

    fn fold_break(&mut self, source: Source) -> Self::Output {
        Ok(break_(source))
    }

    fn fold_if(
        &mut self,
        condition: Expr<Self::Type>,
        if_branch: Vec<Stmt<Self::Type>>,
        else_branch: Option<Vec<Stmt<Self::Type>>>,
        source: Source,
    ) -> Self::Output {
        let condition = self.infer_and_unify(condition, bool())?;
        let if_branch = fold_vec(if_branch, |stmt| self.fold_stmt(stmt))?;
        let else_branch = if let Some(else_branch) = else_branch {
            Some(fold_vec(else_branch, |stmt| self.fold_stmt(stmt))?)
        } else {
            None
        };
        Ok(if_(condition, if_branch, else_branch, source))
    }

    fn fold_while(
        &mut self,
        condition: Expr<Self::Type>,
        body: Vec<Stmt<Self::Type>>,
        source: Source,
    ) -> Self::Output {
        let condition = self.infer_and_unify(condition, bool())?;
        let body = fold_vec(body, |stmt| self.fold_stmt(stmt))?;
        Ok(while_(condition, body, source))
    }

    fn fold_expr_stmt(&mut self, expr: Expr<Self::Type>, source: Source) -> Self::Output {
        let expr = self.infer(expr)?;
        Ok(expr_stmt(expr, source))
    }
}

impl<'a, 'b> StmtVisitor<'a> for SemanticAnalyzer<'b> {
    type Type = Typed;
    type Output = Result<()>;

    fn visit_block(&mut self, statements: &'a [Stmt<Self::Type>]) -> Self::Output {
        for statement in statements {
            self.visit_stmt(statement)?;
        }
        Ok(())
    }

    fn visit_let(
        &mut self,
        _: &'a Identifier,
        _: &'a Option<Typed>,
        initializer: &'a Expr<Self::Type>,
    ) -> Self::Output {
        self.visit_expr(initializer)
    }

    fn visit_return(&mut self, value: &'a Option<Expr<Self::Type>>) -> Self::Output {
        match value {
            Some(value) => self.visit_expr(value),
            None => Ok(()),
        }
    }

    fn visit_break(&mut self) -> Self::Output {
        Ok(())
    }

    fn visit_if(
        &mut self,
        condition: &'a Expr<Self::Type>,
        if_branch: &'a [Stmt<Self::Type>],
        else_branch: &'a Option<Vec<Stmt<Self::Type>>>,
    ) -> Self::Output {
        self.visit_expr(condition)?;
        for stmt in if_branch {
            self.visit_stmt(stmt)?;
        }
        if let Some(else_branch) = else_branch {
            for stmt in else_branch {
                self.visit_stmt(stmt)?;
            }
        }
        Ok(())
    }

    fn visit_while(
        &mut self,
        condition: &'a Expr<Self::Type>,
        body: &'a [Stmt<Self::Type>],
    ) -> Self::Output {
        self.visit_expr(condition)?;
        for stmt in body {
            self.visit_stmt(stmt)?;
        }
        Ok(())
    }

    fn visit_expr_stmt(&mut self, expr: &'a Expr<Self::Type>) -> Self::Output {
        self.visit_expr(expr)
    }
}
