use super::Rule;
use crate::ast::PrimitiveTypes;
use miette::{Diagnostic, SourceOffset};
use pest::iterators::Pair;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Diagnostic)]
pub enum ParserError {
    #[error(transparent)]
    Lexer(Box<pest::error::Error<Rule>>),

    #[error("Empty Input")]
    EmptyInput,

    #[error(transparent)]
    #[diagnostic(transparent)]
    Parser(Source),
}

#[derive(Debug, Error, Diagnostic, PartialEq)]
#[error("unexpexted source code")]
pub struct Source {
    pub cause: Expected,
    #[source_code]
    pub input: String,
    #[label("{cause}")]
    pub location: SourceOffset,
}

#[derive(Debug, Error, Diagnostic, PartialEq)]
pub enum Expected {
    #[error("Expected Type, found nothing")]
    Type,
    #[error("Expected Name, found nothing")]
    Name,

    #[error("Expected Import, found nothing")]
    Import,

    #[error("Expected something, found nothing")]
    Ast,

    #[error("Did not expect a typedef in this place")]
    NotTypeDef,

    #[error("Did not expect a bool in this place")]
    NotBool,

    #[error("Did not expect a import in this place")]
    NotImport,

    #[error("Expected {0:?}, found {1:?}")]
    Found(Rule, Rule),

    #[error("Expected one of {0:?}, found {1:?}")]
    OneOf(Box<[Rule]>, Rule),

    #[error("Expected Integer, found {0:?}")]
    Int(std::num::ParseIntError),

    #[error("Expected Float, found {0:?}")]
    Float(std::num::ParseFloatError),

    #[error("Expected Boolean, found {0:?}")]
    Boolean(String),

    #[error("Expected Boolean, found nothing")]
    Bool,
}

pub(crate) fn expected_pair(expected: Expected, pair: &Pair<Rule>) -> Source {
    let inpt = pair.get_input();
    let (line, col) = pair.line_col();
    Source {
        cause: expected,
        input: inpt.to_string(),
        location: SourceOffset::from_location(inpt, line, col),
    }
}

#[derive(Error, Debug, PartialEq, Diagnostic)]
#[error("type missmatch")]
pub struct CheckError {
    pub cause: (PrimitiveTypes, PrimitiveTypes),
    #[source_code]
    pub input: String,
    #[label("expected {:?}, got {:?}", cause.0, cause.1)]
    pub location: SourceOffset,
}
pub fn type_got(expected: PrimitiveTypes, got: PrimitiveTypes, pair: Pair<Rule>) -> CheckError {
    let inpt = pair.get_input();
    let (line, col) = pair.line_col();
    CheckError {
        cause: (expected, got),
        input: inpt.to_string(),
        location: SourceOffset::from_location(inpt, line, col),
    }
}
