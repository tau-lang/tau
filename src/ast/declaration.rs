use crate::{
    ast::{expression::Expr, identifier::Identifier, statement::Stmt},
    typing::TypeCell,
};
use std::rc::Rc;

pub struct Structure {
    pub name: Identifier,
    pub fields: Vec<(Identifier, TypeCell)>,
    pub methods: Vec<Rc<Decl>>,
}

/// Modifiers are attributes of a element that change it's behaviour.
/// The structure contains a list of modifiers and if they are enabled or
/// not.
#[derive(Default)]
pub struct Modifiers {
    pub is_extern: bool,
    pub is_io: bool,
}

/// A function takes a list of parameters, executes a list of statements and
/// finally returns a expression. The return type is the type of the
/// expression that is returned. A return statement is indicated by the
/// keyword return followed by any expression, possible empty. All occurences
/// of a return must return the same type.
pub struct Function {
    pub name: Identifier,
    pub return_type: TypeCell,
    pub params: Vec<(Identifier, TypeCell)>,
    pub body: Vec<Stmt>,
    pub modifiers: Modifiers,
}

/// A declaration (short decl) denotes a new definition of an element inside
/// a tau file. Every declaration is either a import, struct, function or
/// constant.
pub enum Decl {
    Import(Vec<Identifier>),
    Struct(Structure),
    Function(Function),
    Const {
        name: Identifier,
        var_type: TypeCell,
        initializer: Expr,
    },
}

pub trait DeclVisitor<'a> {
    type Output;

    fn visit_decl(&mut self, decl: &'a Decl) -> Self::Output {
        match decl {
            Decl::Import(name) => self.visit_import(name),
            Decl::Struct(structure) => self.visit_struct(structure),
            Decl::Function(function) => self.visit_function(function),
            Decl::Const {
                name,
                var_type,
                initializer,
            } => self.visit_const(name, var_type, initializer),
        }
    }

    fn visit_import(&mut self, path: &'a [Identifier]) -> Self::Output;

    fn visit_struct(&mut self, structure: &'a Structure) -> Self::Output;

    fn visit_function(&mut self, function: &'a Function) -> Self::Output;

    fn visit_const(
        &mut self,
        name: &'a Identifier,
        var_type: &'a TypeCell,
        initializer: &'a Expr,
    ) -> Self::Output;
}
