use crate::lexer::Source;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    name: String,
    source: Source,
}

impl Identifier {
    pub fn new(name: String, source: Source) -> Self {
        Identifier { name, source }
    }

    pub fn get_source(&self) -> Source {
        self.source.clone()
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

impl Display for Identifier {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        formatter.write_str(&self.name)
    }
}
