use crate::{
    lexer::{Lexer, Source, Token},
    parser::Parser,
};
use std::{
    error,
    fmt::{self, Debug, Display, Formatter},
};

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

pub type Result<T> = std::result::Result<T, Error>;

// FIX: this is not optimal, since i am using the same type for the msg and the cause (that i
// am not creating here)
pub(crate) fn parser_expected(expected: impl ToString, parser: &Parser, next: Token) -> Error {
    Error::new(vec![Diagnostic::new(
        expected.to_string(),
        next.get_source(),
    )])
}

pub(crate) fn lexer_expected(expected: impl ToString, lexer: &Lexer) -> Error {
    let (line, col) = lexer.location();
    Error::new(vec![Diagnostic::new(
        expected.to_string(),
        Source::new(lexer.file(), line, col),
    )])
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

// #[derive(Debug)]
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
            std::fmt::Display::fmt(cause, formatter)?;
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
