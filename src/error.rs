use crate::lexer::{Lexer, Source, Token};
use std::{
    error,
    fmt::{self, Debug, Display, Formatter},
};

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

    pub fn after(self, prev: Option<Self>) -> Option<Self> {
        if let Some(err) = prev {
            return Some(err.concat(self));
        }
        Some(self)
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
}

impl Diagnostic {
    pub fn new(message: String, source: Source) -> Diagnostic {
        Diagnostic { message, source }
    }
}

impl Display for Diagnostic {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::result::Result<(), fmt::Error> {
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
