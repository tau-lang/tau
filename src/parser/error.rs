use thiserror::Error;

use super::Rule;

#[derive(Error, Debug, PartialEq)]
pub enum ParserError {
    #[error("Error while lexing")]
    Lexer(pest::error::Error<Rule>),
    #[error("Empty Input")]
    EmptyInput,
}
