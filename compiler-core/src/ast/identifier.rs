use crate::{ast::Location, lexer::Source};
use std::fmt::{Display, Formatter, Result};
use symbol_table::GlobalSymbol;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Identifier {
    /// A global symbol is a interned string in the global symbol table.
    /// Multiple symbols refer to the same string, which allows us to remove
    /// useless clone calls without introducing live times.
    ///
    /// <https://docs.rs/symbol_table/latest/symbol_table/struct.GlobalSymbol.html>
    name: GlobalSymbol,
    source: Source,
}

impl Identifier {
    pub fn new(name: GlobalSymbol, source: Source) -> Self {
        Identifier { name, source }
    }

    pub fn name(&self) -> &str {
        &self.name.as_str()
    }

    pub fn symbol(&self) -> GlobalSymbol {
        self.name
    }
}

impl Location for Identifier {
    fn source(&self) -> Source {
        self.source
    }
}

impl Display for Identifier {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        formatter.write_str(&self.name())
    }
}

impl From<Identifier> for GlobalSymbol {
    fn from(value: Identifier) -> GlobalSymbol {
        value.name
    }
}
