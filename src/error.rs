use crate::lexer::{Lexer, Source, Token};
use std::{
    error,
    fmt::{self, Debug, Display, Formatter},
};

use std::fs;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn parser_expected(expected: impl ToString, next: Token) -> Error {
    Error::new(vec![Diagnostic::new(
        expected.to_string(),
        next.get_source(),
    )])
}

pub(crate) fn lexer_expected(expected: impl ToString, lexer: &Lexer) -> Error {
    Error::new(vec![Diagnostic::new(
        expected.to_string(),
        lexer.make_source(),
    )])
}

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const YELLOW: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const BLUE: &str = "\x1b[34m";
const GREEN: &str = "\x1b[93m";

pub struct Error(Vec<Diagnostic>);

impl Error {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Error(diagnostics)
    }
    pub fn concat(self, next: Self) -> Self {
        let mut m = self.0;
        let mut n = next.0;
        m.append(&mut n);
        Error(m)
    }
}

impl error::Error for Error {}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::result::Result<(), fmt::Error> {
        for diagnostic in &self.0 {
            std::fmt::Display::fmt(diagnostic, formatter)?;
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
pub struct Cause(String, Source);

impl Display for Cause {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {} in {}", self.0, self.1.line(), self.1.file())
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
            std::fmt::Display::fmt(cause, formatter)?;
        }
        write!(
            formatter,
            "
 {BOLD}{BLUE}-->{RESET} {}
  {BOLD}{BLUE}|
  |{RESET} {}
  {BOLD}{BLUE}|{RESET}{RED}  ^ {}{RESET}",
            self.source,
            self.source.content().expect("File exists"),
            self.message
        )
    }
}
