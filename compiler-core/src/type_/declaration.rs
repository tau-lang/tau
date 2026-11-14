use crate::{
    analyzer::SemanticAnalyzer,
    ast::{
        Identifier, TypedDecl,
        declaration::{DeclFolder, FuncDecl, StructDecl},
        expression::Expr,
        fold_vec,
        prelude::*,
        statement::{StmtFolder, StmtVisitor},
    },
    error::Result,
    lexer::Source,
    type_::{Typed, Untyped},
};
use std::collections::HashMap;

impl<'a> DeclFolder for SemanticAnalyzer<'a> {
    type Type = Untyped;
    type Output = Result<TypedDecl>;

    fn fold_import(&mut self, path: Vec<Identifier>, source: Source) -> Self::Output {
        Ok(import(path, source))
    }

    fn fold_struct(&mut self, s: StructDecl, source: Source) -> Self::Output {
        Ok(struct_(s, source))
    }

    fn fold_function(&mut self, f: FuncDecl<Self::Type>, source: Source) -> Self::Output {
        let (name, return_type, params, body, modifiers) =
            (f.name, f.return_type, f.params, f.body, f.modifiers);
        self.environment.begin_scope(HashMap::new());
        for (name, type_) in &params {
            self.environment.declare_var(name, type_.clone())?
        }
        self.environment.set_return_type(return_type.clone());
        let body = fold_vec(body, |stmt| self.fold_stmt(stmt))?;
        for stmt in &body {
            self.visit_stmt(stmt)?;
        }
        self.environment.end_scope();
        Ok(func(
            FuncDecl::new(name, return_type, params, body, modifiers),
            source,
        ))
    }

    fn fold_const(
        &mut self,
        name: Identifier,
        type_: Typed,
        initializer: Expr<Self::Type>,
        source: Source,
    ) -> Self::Output {
        let initializer = self.infer_and_unify(initializer, type_.clone())?;
        Ok(const_(name, type_, initializer, source))
    }
}
