use miette::{Diagnostic, SourceOffset};
use thiserror::Error;

use crate::{
    lexer::{Token, TokenType},
    parser::Parser,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug, PartialEq, Diagnostic)]
pub enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Lexer(#[from] LexError),

    #[error("Empty Input")]
    EmptyInput,

    #[error(transparent)]
    #[diagnostic(transparent)]
    Parser(#[from] Source),
}

#[derive(Debug, Error, Diagnostic, PartialEq)]
#[error("unexpexted source code: {cause}")]
pub struct Source {
    pub cause: Expected,
    #[source_code]
    pub input: String,
    #[label(primary, "{cause}")]
    pub location: SourceOffset,
}

pub(crate) fn expected(expected: Expected, parser: &Parser, next: Token) -> Error {
    let (line, col) = next.get_offset();
    Error::Parser(Source {
        cause: expected,
        location: SourceOffset::from_location(parser.get_source(), line as usize, col as usize),
        input: parser.get_source().to_string(),
    })
}

#[allow(dead_code)]
#[derive(Debug, Error, Diagnostic, PartialEq)]
pub enum Expected {
    #[error("Unecpected Token \"{0:?}\", expected one of {1:?}")]
    UnexpectedToken(TokenType, String),
    #[error("Expected a specific token (\"{0:?}\"), found \"{1:?}\"")]
    SpecificTokenType(TokenType, TokenType),
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

    #[error("Did not expect a import in this place")]
    NotImport,

    // #[error("Expected {0:?}, found {1:?}")]
    // Found(Rule, Rule),

    // #[error("Expected one of {0:?}, found {1:?}")]
    // OneOf(Box<[Rule]>, Rule),
    #[error("Expected Integer, found {0:?}")]
    Int(std::num::ParseIntError),

    #[error("Expected Float, found {0:?}")]
    Float(std::num::ParseFloatError),

    #[error("Expected Boolean, found {0:?}")]
    Boolean(String),

    #[error("Expected Boolean, found nothing")]
    Bool,
}

// pub(crate) fn expected_pair(expected: Expected, pair: &Pair<Rule>) -> Source {
//     let inpt = pair.get_input();
//     let (line, col) = pair.line_col();
//     Source {
//         cause: expected,
//         input: inpt.to_string(),
//         location: SourceOffset::from_location(inpt, line, col),
//     }
// }

// #[derive(Error, Debug, PartialEq, Diagnostic)]
// #[error("type missmatch")]
// pub struct CheckError {
//     pub cause: (TokenType, PrimitiveTypes),
//     #[source_code]
//     pub input: String,
//     #[label("expected {:?}, got {:?}", cause.0, cause.1)]
//     pub location: SourceOffset,
// }
// pub fn type_got(expected: PrimitiveTypes, got: PrimitiveTypes, pair: Pair<Rule>) -> CheckError {
//     let inpt = pair.get_input();
//     let (line, col) = pair.line_col();
//     CheckError {
//         cause: (expected, got),
//         input: inpt.to_string(),
//         location: SourceOffset::from_location(inpt, line, col),
//     }
// }

#[derive(Error, Debug, PartialEq, Diagnostic)]
#[error("unexpected token")]
pub struct LexError {
    pub cause: LexErrorVarient,
    #[source_code]
    pub input: String,
    #[label("error at {cause}")]
    pub location: SourceOffset,
}

#[derive(Error, Debug, PartialEq, Diagnostic)]
pub enum LexErrorVarient {
    #[error("custom lexing error")]
    Custom(String),
    // #[error("error processing input, expected a {0:?}")]
    // Parsing(Rule),
}
