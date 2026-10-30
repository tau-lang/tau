use crate::{
    ast::{Location, Typed, expression::Expr, identifier::Identifier, statement::Stmt},
    lexer::Source,
};

#[derive(Debug)]
pub struct DeclStruct {
    pub name: Identifier,
    pub fields: Vec<(Identifier, Typed)>,
}

impl DeclStruct {
    pub fn new(name: Identifier, fields: Vec<(Identifier, Typed)>) -> Self {
        Self { name, fields }
    }
}

/// Modifiers are attributes of a element that change it's behavior. They are
/// written before an function, structure or parameter declaration to add
/// additional functionality to it. Currently only the extern and io modifier
/// exists. The structure contains a list of modifiers and if they are enabled
/// or not.
#[derive(Debug, Default)]
pub struct FuncModifiers {
    pub is_extern: bool,
    pub is_io: bool,
}

impl FuncModifiers {
    pub fn new(is_extern: bool, is_io: bool) -> Self {
        Self { is_extern, is_io }
    }
}

/// A function takes a list of parameters, executes a list of statements and
/// finally returns a expression. The return type is the type of the
/// expression that is returned. A return statement is indicated by the
/// keyword return followed by any expression, possible empty. All occurences
/// of a return must return the same type.
#[derive(Debug)]
pub struct DeclFunc<T> {
    pub name: Identifier,
    pub return_type: Typed,
    pub params: Vec<(Identifier, Typed)>,
    pub body: Vec<Stmt<T>>,
    pub modifiers: FuncModifiers,
}

impl<T> DeclFunc<T> {
    pub fn new(
        name: Identifier,
        return_type: Typed,
        params: Vec<(Identifier, Typed)>,
        body: Vec<Stmt<T>>,
        modifiers: FuncModifiers,
    ) -> Self {
        Self {
            name,
            return_type,
            params,
            body,
            modifiers,
        }
    }
}

/// A declaration (short decl) denotes a new definition of an element inside
/// a tau file.
#[derive(Debug)]
pub struct Decl<T> {
    kind: DeclKind<T>,
    source: Source,
}

impl<T> Decl<T> {
    pub fn new(kind: DeclKind<T>, source: Source) -> Decl<T> {
        Decl { kind, source }
    }

    pub fn kind(&self) -> &DeclKind<T> {
        &self.kind
    }
}

impl<T> Location for Decl<T> {
    fn source(&self) -> Source {
        self.source
    }
}

/// Every declaration is either a import, struct, function or constant.
#[derive(Debug)]
pub enum DeclKind<T> {
    Import(Vec<Identifier>),
    Struct(DeclStruct),
    Func(DeclFunc<T>),
    Const {
        name: Identifier,
        type_: Typed,
        initializer: Expr<T>,
    },
}

pub trait DeclVisitor<'a> {
    type Type;
    type Output;

    fn visit_decl(&mut self, decl: &'a Decl<Self::Type>) -> Self::Output {
        match decl.kind() {
            DeclKind::Import(name) => self.visit_import(name, decl.source()),
            DeclKind::Struct(structure) => self.visit_struct(structure, decl.source()),
            DeclKind::Func(function) => self.visit_function(function, decl.source()),
            DeclKind::Const {
                name,
                type_,
                initializer,
            } => self.visit_const(name, type_, initializer, decl.source()),
        }
    }

    fn visit_import(&mut self, path: &'a [Identifier], source: Source) -> Self::Output;

    fn visit_struct(&mut self, struct_: &'a DeclStruct, source: Source) -> Self::Output;

    fn visit_function(&mut self, func: &'a DeclFunc<Self::Type>, source: Source) -> Self::Output;

    fn visit_const(
        &mut self,
        name: &'a Identifier,
        type_: &'a Typed,
        initializer: &'a Expr<Self::Type>,
        source: Source,
    ) -> Self::Output;
}

pub trait DeclFolder {
    type Type;
    type Output;

    fn fold_decl(&mut self, decl: Decl<Self::Type>) -> Self::Output {
        let (kind, source) = (decl.kind, decl.source);
        match kind {
            DeclKind::Import(name) => self.fold_import(name, source),
            DeclKind::Struct(structure) => self.fold_struct(structure, source),
            DeclKind::Func(function) => self.fold_function(function, source),
            DeclKind::Const {
                name,
                type_,
                initializer,
            } => self.fold_const(name, type_, initializer, source),
        }
    }

    fn fold_import(&mut self, path: Vec<Identifier>, source: Source) -> Self::Output;

    fn fold_struct(&mut self, struct_: DeclStruct, source: Source) -> Self::Output;

    fn fold_function(&mut self, func: DeclFunc<Self::Type>, source: Source) -> Self::Output;

    fn fold_const(
        &mut self,
        name: Identifier,
        type_: Typed,
        initializer: Expr<Self::Type>,
        source: Source,
    ) -> Self::Output;
}
