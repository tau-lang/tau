use miette::{Diagnostic, SourceOffset};
use thiserror::Error;

use crate::{
    lexer::{Lexer, Token, TokenType},
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

pub(crate) fn parser_expected(expected: Expected, parser: &Parser, next: Token) -> Error {
    let (line, col) = next.get_offset();
    Error::Parser(Source {
        cause: expected,
        location: SourceOffset::from_location(parser.get_source(), line as usize, col as usize),
        input: parser.get_source().to_string(),
    })
}

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

    #[error("Expected Integer, found {0:?}")]
    Int(std::num::ParseIntError),

    #[error("Expected Float, found {0:?}")]
    Float(std::num::ParseFloatError),

    #[error("Expected Boolean, found {0:?}")]
    Boolean(String),

    #[error("Expected Boolean, found nothing")]
    Bool,
}

#[derive(Error, Debug, PartialEq, Diagnostic)]
#[error("unexpected token")]
pub struct LexError {
    pub cause: LexErrorVarient,
    #[source_code]
    pub input: String,
    #[label("error at {cause}")]
    pub location: SourceOffset,
}

pub(crate) fn lexer_expected(expected: LexErrorVarient, lexer: &Lexer) -> Error {
    let (line, col) = lexer.location();
    Error::Lexer(LexError {
        cause: expected,
        location: SourceOffset::from_location(lexer.source(), line, col),
        input: lexer.source(),
    })
    // Error::Parser(Source {
    //     cause: expected,
    //     location: SourceOffset::from_location(parser.get_source(), line as usize, col as usize),
    //     input: parser.get_source().to_string(),
    // })
}

#[derive(Error, Debug, PartialEq, Diagnostic)]
pub enum LexErrorVarient {
    #[error("custom lexing error")]
    Custom(String),

    #[error("unexpected EOF, expected {0}")]
    UnexpectedEoF(String),

    #[error("encountered a second dot in a float")]
    FloatSecondDot,
}
