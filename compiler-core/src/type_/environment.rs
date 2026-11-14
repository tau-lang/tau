use crate::{
    ast::{Location, Unfold, identifier::Identifier, lookup::Lookup, origin::Origin},
    error::{Error, Result, diagnostic, error, label},
    lexer::Source,
    type_::{Typed, prelude::*, uid::UniqueIdGenerator},
};
use std::{collections::HashMap, sync::Arc};
use symbol_table::GlobalSymbol;

pub type TypeDefinition = Origin<Typed>;
pub type Scope = HashMap<GlobalSymbol, TypeDefinition>;

pub trait Namespace {
    fn find_symbol(&self, name: &Identifier) -> Result<&TypeDefinition>;

    fn find_subspace(&self, name: &Identifier) -> Result<&dyn Namespace>;
}

#[derive(Debug)]
pub struct StructInterface {
    /// The name of this struct.
    name: Identifier,

    /// Attributes
    items: Scope,

    /// All associated members like static constants and methods.
    associated: Scope,
}

impl StructInterface {
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            items: Scope::new(),
            associated: Scope::new(),
        }
    }

    pub fn declare_item(&mut self, name: &Identifier, type_: Typed) -> Result<()> {
        self.items
            .insert(name.symbol(), TypeDefinition::new(type_, name.source()))
            .map_or(Ok(()), |def| {
                error(
                    diagnostic("field already exists", name)
                        .with_label(label("variable was already declared here", &def)),
                )
            })
    }

    pub fn find_item(&self, name: &Identifier) -> Result<&TypeDefinition> {
        self.items
            .get(&name.symbol())
            .ok_or_else(|| Error::new(diagnostic("field not found", name)))
    }

    pub fn declare_associated(&mut self, name: &Identifier, type_: Typed) -> Result<()> {
        self.associated
            .insert(name.symbol(), TypeDefinition::new(type_, name.source()))
            .map_or(Ok(()), |def| {
                error(
                    diagnostic("field already exists", name)
                        .with_label(label("variable was already declared here", &def)),
                )
            })
    }

    pub fn find_associated(&self, name: &Identifier) -> Result<&TypeDefinition> {
        self.associated
            .get(&name.symbol())
            .ok_or_else(|| Error::new(diagnostic("field not found", name)))
    }

    pub fn name(&self) -> &Identifier {
        &self.name
    }
}

impl Namespace for StructInterface {
    fn find_symbol(&self, name: &Identifier) -> Result<&TypeDefinition> {
        self.find_associated(name)
    }

    fn find_subspace(&self, name: &Identifier) -> Result<&dyn Namespace> {
        error(
            diagnostic("structs can not contain namespaces", name).with_label(label(
                format!("when trying to access struct {}", self.name),
                self,
            )),
        )
    }
}

impl Location for StructInterface {
    fn source(&self) -> Source {
        self.name.source()
    }
}

#[derive(Debug)]
pub struct ModuleInterface {
    /// The name of this module.
    name: Identifier,

    /// Available submodules.
    modules: HashMap<GlobalSymbol, Arc<ModuleInterface>>,

    /// Declared structs.
    structs: HashMap<GlobalSymbol, StructInterface>,

    /// Associated types, functions and variables.
    associated: Scope,
}

impl ModuleInterface {
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            modules: HashMap::new(),
            structs: HashMap::new(),
            associated: Scope::new(),
        }
    }

    pub fn declare_module(&mut self, module: Arc<ModuleInterface>) -> Result<()> {
        self.modules
            .insert(module.as_ref().name().symbol(), module)
            .map_or(Ok(()), |d| {
                error(diagnostic("struct already exists", d.name()))
            })
    }

    pub fn find_module(&self, name: &Identifier) -> Result<&Arc<ModuleInterface>> {
        self.modules
            .get(&name.symbol())
            .ok_or_else(|| Error::new(diagnostic("module not found", name)))
    }

    pub fn declare_struct(&mut self, struct_: StructInterface) -> Result<()> {
        self.structs
            .insert(struct_.name().symbol(), struct_)
            .map_or(Ok(()), |d| error(diagnostic("struct already exists", &d)))
    }

    pub fn find_struct(&self, name: &GlobalSymbol) -> Option<&StructInterface> {
        self.structs.get(name)
    }

    pub fn declare_associated(&mut self, name: &Identifier, type_: Typed) -> Result<()> {
        self.associated
            .insert(name.symbol(), TypeDefinition::new(type_, name.source()))
            .map_or(Ok(()), |def| {
                error(
                    diagnostic("name already exists", name)
                        .with_label(label("name was already declared here", &def)),
                )
            })
    }

    pub fn find_associated(&self, name: &Identifier) -> Result<&TypeDefinition> {
        self.associated
            .get(&name.symbol())
            .ok_or_else(|| Error::new(diagnostic("field not found", name)))
    }

    pub fn name(&self) -> &Identifier {
        &self.name
    }
}

impl Namespace for ModuleInterface {
    fn find_symbol(&self, name: &Identifier) -> Result<&TypeDefinition> {
        self.find_associated(name)
    }

    fn find_subspace(&self, name: &Identifier) -> Result<&dyn Namespace> {
        if let Some(module) = self.modules.get(&name.symbol()) {
            return Ok(module.as_ref() as &dyn Namespace);
        }
        if let Some(struct_) = self.structs.get(&name.symbol()) {
            return Ok(struct_ as &dyn Namespace);
        }
        error(diagnostic("name not found", name))
    }
}

#[derive(Debug)]
pub struct Environment {
    ids: UniqueIdGenerator,

    /// The module of the current file.
    module: Arc<ModuleInterface>,

    /// Values defined in the current function
    scopes: Vec<Scope>,

    /// Return type of the current function
    return_type: Typed,
}

impl Environment {
    pub fn new(ids: UniqueIdGenerator, module: Arc<ModuleInterface>) -> Self {
        Self {
            ids,
            module,
            scopes: Vec::new(),
            return_type: void(),
        }
    }

    pub fn begin_scope(&mut self, scope: Scope) {
        self.scopes.push(scope);
    }

    pub fn end_scope(&mut self) {
        self.scopes
            .pop()
            .expect("same number of open scopes as closed scopes");
    }

    fn scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().expect("scope exists")
    }

    fn module(&self) -> &ModuleInterface {
        &self.module
    }

    pub fn find_struct(&self, name: &GlobalSymbol) -> Option<&StructInterface> {
        self.module().find_struct(name)
    }

    pub fn declare_var(&mut self, name: &Identifier, type_: Typed) -> Result<()> {
        self.scope_mut()
            .insert(name.symbol(), TypeDefinition::new(type_, name.source()))
            .map_or(Ok(()), |def| {
                error(
                    diagnostic("variable already exists", name)
                        .with_label(label("variable was already declared here", &def)),
                )
            })
    }

    pub fn find_symbol(&self, lookup: &Lookup, name: &Identifier) -> Result<&TypeDefinition> {
        match lookup {
            Lookup::Qualified(path) => {
                let mut space = self.module() as &dyn Namespace;
                for name in path {
                    space = space.find_subspace(name)?;
                }
                space.find_symbol(name)
            }
            Lookup::Unqualified => self.module().find_symbol(name),
        }
    }

    pub fn find_var(&self, lookup: &Lookup, name: &Identifier) -> Result<&TypeDefinition> {
        match lookup {
            Lookup::Qualified(path) => {
                let mut space = self.module() as &dyn Namespace;
                for name in path {
                    space = space.find_subspace(name)?;
                }
                space.find_symbol(name)
            }
            Lookup::Unqualified => {
                for scope in self.scopes.iter().rev() {
                    if let Some(value) = scope.get(&name.symbol()) {
                        return Ok(value);
                    }
                }

                self.module().find_symbol(name)
            }
        }
    }

    pub fn new_unbound_var(&mut self) -> Typed {
        unbound_var(self.ids.next())
    }

    pub fn return_type(&self) -> &Typed {
        &self.return_type
    }

    pub fn set_return_type(&mut self, type_: Typed) {
        self.return_type = type_;
    }
}

impl Unfold for Environment {
    type Unfolded = Arc<ModuleInterface>;

    fn unfold(self) -> Self::Unfolded {
        self.module
    }
}
