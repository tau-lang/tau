use crate::{
    lexer::{Lexer, Source, Token, TokenType},
    parser::Parser,
};
use std::{
    error,
    fmt::{self, Display, Formatter},
};
use thiserror::Error;

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn parser_expected(expected: ParserError, parser: &Parser, next: Token) -> Error {
    // let (line, col) = next.get_offset();
    todo!()
    // Error::Parser(Source {
    //     cause: expected,
    //     location: SourceOffset::from_location(parser.get_source(), line as usize, col as usize),
    //     input: parser.get_source().to_string(),
    // })
}
//
#[derive(Debug, Error, PartialEq)]
pub enum Expected {
    #[error(transparent)]
    Parse(ParserError),
    #[error(transparent)]
    Lex(LexError),
}
#[derive(Debug, Error, PartialEq)]
pub enum LexError {
    #[error("custom lexing error")]
    Custom(String),

    #[error("unexpected EOF, expected {0}")]
    UnexpectedEoF(String),

    #[error("encountered a second dot in a float")]
    FloatSecondDot,
}
#[derive(Debug, Error, PartialEq)]
pub enum ParserError {
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

pub(crate) fn lexer_expected(expected: LexError, lexer: &Lexer) -> Error {
    let (line, col) = lexer.location();
    todo!()
    // Error::Lexer(LexError {
    //     cause: expected,
    //     location: SourceOffset::from_location(lexer.source(), line, col),
    //     input: lexer.source(),
    // })
    // Error::Parser(Source {
    //     cause: expected,
    //     location: SourceOffset::from_location(parser.get_source(), line as usize, col as usize),
    //     input: parser.get_source().to_string(),
    // })
}

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const YELLOW: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const BLUE: &str = "\x1b[34m";
const GREEN: &str = "\x1b[93m";

fn nth_line<P: AsRef<Path>>(path: P, n: usize) -> io::Result<Option<String>> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    for (i, line) in reader.lines().enumerate() {
        if i + 1 == n {
            return line.map(Some);
        }
    }

    Ok(None) // File had fewer than n lines
}

#[derive(Debug)]
pub struct Error(Vec<Diagnostic>);

impl Error {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Error { 0: diagnostics }
    }
}

impl error::Error for Error {}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::result::Result<(), fmt::Error> {
        for diagnostic in &self.0 {
            diagnostic.fmt(formatter)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Diagnostic {
    message: String,
    source: Source,
    cause: Option<Cause>,
}

#[derive(Debug)]
pub struct Cause(Expected, Source);

impl Display for Cause {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}:{} in {}",
            self.0,
            self.1.line(),
            self.1.column(),
            self.1.file()
        )
    }
}

impl Diagnostic {
    pub fn new(message: String, source: Source) -> Diagnostic {
        Diagnostic {
            message,
            source,
            cause: None,
        }
    }

    pub fn with_cause(message: String, source: Source, cause: Cause) -> Diagnostic {
        Diagnostic {
            message,
            source,
            cause: Some(cause),
        }
    }
}

impl Display for Diagnostic {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::result::Result<(), fmt::Error> {
        if let Some(cause) = &self.cause {
            cause.fmt(formatter)?;
        }
        write!(
            formatter,
            "
 {BOLD}{BLUE}-->{RESET} {}
  {BOLD}{BLUE}|
  |{RESET} {}
  {BOLD}{BLUE}|{RESET}{RED}{}^ {}{RESET}",
            self.source,
            nth_line(self.source.file(), self.source.line())
                .unwrap()
                .unwrap(),
            " ".repeat(self.source.column()),
            self.message
        )
    }
}
