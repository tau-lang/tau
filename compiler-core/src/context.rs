use crate::{
    error::{DynResult, Error, FileDb, FileId},
    type_::environment::ModuleInterface,
    uid::UniqueIdGenerator,
};
use codespan_reporting::{
    diagnostic::Severity,
    term::{
        self, Config,
        termcolor::{ColorChoice, StandardStream},
    },
};
use std::{collections::HashMap, sync::Arc};
use symbol_table::GlobalSymbol;

pub struct Context {
    files: FileDb,
    writer: StandardStream,
    config: Config,
    /// Id generator for unbound types.
    ids: UniqueIdGenerator,
    /// All modules that were read with this context.
    modules: HashMap<GlobalSymbol, Arc<ModuleInterface>>,
    /// Highest severity that were handled by this context.
    severity: Option<Severity>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            files: FileDb::new(),
            writer: StandardStream::stderr(ColorChoice::Always),
            config: Config::default(),
            ids: UniqueIdGenerator::new(),
            modules: HashMap::new(),
            severity: None,
        }
    }

    pub fn handle(&mut self, error: &Error) -> DynResult {
        for diagnostic in error.diagnostics() {
            self.severity = Some(
                self.severity
                    .map_or_else(|| diagnostic.severity, |s| s.max(diagnostic.severity)),
            );
            term::emit_to_io_write(
                &mut self.writer.lock(),
                &self.config,
                &self.files,
                diagnostic,
            )
            .map_err(|e| Box::new(e) as Box<dyn std::fmt::Debug>)?;
        }
        Ok(())
    }

    pub fn store(&mut self, name: String, source: String) -> FileId {
        self.files.add(name, source)
    }

    pub fn ids(&self) -> &UniqueIdGenerator {
        &self.ids
    }

    pub fn cache_module(&mut self, module: Arc<ModuleInterface>) {
        self.modules.insert(module.name(), module);
    }

    pub fn find_module(&self, name: &GlobalSymbol) -> Option<&Arc<ModuleInterface>> {
        self.modules.get(name)
    }

    pub fn severity(&self) -> Option<Severity> {
        self.severity
    }
}
