use crate::{
    ast::{Location, identifier::Identifier},
    error::{Error, Result, diagnostic, error, label},
    lexer::Source,
    type_::{Typed, prelude::*},
    uid::UniqueIdGenerator,
};
use std::{collections::HashMap, sync::Arc};
use symbol_table::GlobalSymbol;

#[derive(Debug)]
pub struct TypeDefinition {
    type_: Typed,
    source: Source,
}

impl TypeDefinition {
    pub fn new(type_: Typed, source: Source) -> Self {
        Self { type_, source }
    }

    pub fn type_(&self) -> &Typed {
        &self.type_
    }
}

impl Location for TypeDefinition {
    fn source(&self) -> Source {
        self.source
    }
}

pub type Scope = HashMap<GlobalSymbol, TypeDefinition>;

#[derive(Debug)]
pub struct StructInterface {
    /// The name of this struct.
    name: GlobalSymbol,

    /// All associated members like attributes and methods.
    associated: Scope,

    /// Where this struct was declared.
    source: Source,
}

impl StructInterface {
    pub fn new(name: GlobalSymbol, source: Source) -> Self {
        Self {
            name,
            associated: Scope::new(),
            source,
        }
    }

    pub fn declare_associaten(&mut self, name: &Identifier, type_: Typed) -> Result<()> {
        self.associated
            .insert(name.symbol(), TypeDefinition::new(type_, name.source()))
            .map_or_else(
                || Ok(()),
                |def| {
                    error(
                        diagnostic("field already exists", name)
                            .with_label(label("variable was already declared here", &def)),
                    )
                },
            )
    }

    pub fn find_associaten(&self, name: &Identifier) -> Result<&TypeDefinition> {
        self.associated
            .get(&name.symbol())
            .ok_or_else(|| Error::new(vec![diagnostic("field not found", name)]))
    }

    pub fn name(&self) -> GlobalSymbol {
        self.name
    }
}

impl Location for StructInterface {
    fn source(&self) -> Source {
        self.source
    }
}

#[derive(Debug)]
pub struct ModuleInterface {
    /// The name of this module.
    name: GlobalSymbol,

    /// Available submodules.
    #[allow(unused)]
    modules: HashMap<GlobalSymbol, Arc<ModuleInterface>>,

    /// Declared structs.
    structs: HashMap<GlobalSymbol, StructInterface>,

    /// Declared types.
    types: Scope,

    /// Declared static values.
    values: Scope,
}

impl ModuleInterface {
    pub fn new(name: GlobalSymbol) -> Self {
        Self {
            name,
            modules: HashMap::new(),
            structs: HashMap::new(),
            types: Scope::new(),
            values: Scope::new(),
        }
    }

    pub fn declare_struct(&mut self, struct_: StructInterface) -> Result<()> {
        self.structs.insert(struct_.name(), struct_).map_or_else(
            || Ok(()),
            |d| error(diagnostic("struct already exists", &d)),
        )
    }

    pub fn find_struct(&self, name: &GlobalSymbol) -> Option<&StructInterface> {
        self.structs.get(name)
    }

    pub fn declare_var(&mut self, name: &Identifier, type_: Typed) -> Result<()> {
        self.values
            .insert(name.symbol(), TypeDefinition::new(type_, name.source()))
            .map_or_else(
                || Ok(()),
                |def| {
                    error(
                        diagnostic("variable already exists", name)
                            .with_label(label("variable was already declared here", &def)),
                    )
                },
            )
    }

    pub fn find_var(&self, name: &Identifier) -> Result<&TypeDefinition> {
        self.values
            .get(&name.symbol())
            .ok_or_else(|| Error::new(vec![diagnostic("variable not found", name)]))
    }

    pub fn declare_type(&mut self, name: &Identifier, type_: Typed) -> Result<()> {
        self.types
            .insert(name.symbol(), TypeDefinition::new(type_, name.source()))
            .map_or_else(
                || Ok(()),
                |def| {
                    error(
                        diagnostic("variable already exists", name)
                            .with_label(label("variable was already declared here", &def)),
                    )
                },
            )
    }

    pub fn find_type(&self, name: &Identifier) -> Result<&TypeDefinition> {
        self.types
            .get(&name.symbol())
            .ok_or_else(|| Error::new(vec![diagnostic("variable not found", name)]))
    }

    pub fn name(&self) -> GlobalSymbol {
        self.name
    }
}

#[derive(Debug)]
pub struct Environment {
    ids: UniqueIdGenerator,

    /// Modules that have been imported by the current module.
    modules: HashMap<GlobalSymbol, Arc<ModuleInterface>>,

    /// Values defined in the current function
    scopes: Vec<Scope>,

    /// The module of the current file.
    current_module: GlobalSymbol,
}

impl Environment {
    pub fn new(ids: UniqueIdGenerator, module: Arc<ModuleInterface>) -> Self {
        Self {
            ids,
            current_module: module.name(),
            modules: {
                let mut modules = HashMap::new();
                modules.insert(module.name(), module);
                modules
            },
            scopes: Vec::new(),
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
        self.modules
            .get(&self.current_module)
            .expect("own module exists")
    }

    pub fn find_struct(&self, name: &GlobalSymbol) -> Option<&StructInterface> {
        self.module().find_struct(name)
    }

    pub fn declare_var(&mut self, name: &Identifier, type_: Typed) -> Result<()> {
        self.scope_mut()
            .insert(name.symbol(), TypeDefinition::new(type_, name.source()))
            .map_or_else(
                || Ok(()),
                |def| {
                    error(
                        diagnostic("variable already exists", name)
                            .with_label(label("variable was already declared here", &def)),
                    )
                },
            )
    }

    pub fn find_var(&self, name: &Identifier) -> Result<&TypeDefinition> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(&name.symbol()) {
                return Ok(value);
            }
        }

        self.module().find_var(name)
    }

    pub fn find_type(&self, name: &Identifier) -> Result<&TypeDefinition> {
        self.module().find_type(name)
    }

    pub fn new_unbound_var(&mut self) -> Typed {
        unbound_var(self.ids.next())
    }
}
