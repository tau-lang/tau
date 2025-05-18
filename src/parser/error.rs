use super::Rule;
use miette::{Diagnostic, SourceOffset};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Diagnostic)]
pub enum ParserError {
    #[error(transparent)]
    Lexer(pest::error::Error<Rule>),

    #[error("Empty Input")]
    EmptyInput,

    #[error(transparent)]
    #[diagnostic(transparent)]
    Parser(Source),
}

#[derive(Debug, Error, Diagnostic, PartialEq)]
#[error("unexpexted source code")]
pub struct Source {
    pub cause: Unexpected,
    #[source_code]
    pub input: String,
    #[label("{cause}")]
    pub location: SourceOffset,
}

#[derive(Debug, Error, Diagnostic, PartialEq)]
pub enum Unexpected {
    #[error("eee")]
    Eeee,
}
