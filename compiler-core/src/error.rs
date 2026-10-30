use crate::lexer::Source;
use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::SimpleFiles,
};
use std::{
    fmt::{Debug, Display},
    result,
};

pub type DynResult = std::result::Result<(), Box<dyn std::fmt::Debug>>;

/// All results throw our error type.
pub type Result<T> = result::Result<T, Error>;

/// The file database that we currently use. This may change in the future.
pub type FileDb = SimpleFiles<String, String>;
pub type FileId = usize; // <FileDb as Files>::FileId

#[derive(Debug)]
pub struct Error(Vec<Diagnostic<FileId>>);

impl Error {
    pub fn new(diagnostics: Vec<Diagnostic<FileId>>) -> Self {
        Error(diagnostics)
    }

    pub fn with_diagnostic(diagnostic: Diagnostic<FileId>) -> Self {
        Self(vec![diagnostic])
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

    pub fn diagnostics(&self) -> &Vec<Diagnostic<FileId>> {
        &self.0
    }
}

pub fn diagnostic(message: impl Display, source: impl Into<Source>) -> Diagnostic<FileId> {
    let source = source.into();
    Diagnostic::error()
        .with_message(message)
        .with_label(Label::primary(source.file_id(), source.range()))
}

pub fn primary(source: impl Into<Source>) -> Label<FileId> {
    let source = source.into();
    Label::primary(source.file_id(), source.range())
}

pub fn label(message: impl Display, source: impl Into<Source>) -> Label<FileId> {
    let source = source.into();
    Label::secondary(source.file_id(), source.range()).with_message(message)
}

pub fn error<T>(diagnostic: Diagnostic<FileId>) -> Result<T> {
    Err(Error(vec![diagnostic]))
}
