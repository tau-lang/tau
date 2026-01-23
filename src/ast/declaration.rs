use crate::{
    ast::{expression::Expr, identifier::Identifier, statement::Stmt},
    typing::TypeCell,
};
use std::rc::Rc;

#[derive(Debug)]
pub enum Decl {
    Import(Vec<Identifier>),
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

    fn visit_import(&mut self, path: &'a [Identifier]) -> T;

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
