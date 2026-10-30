use crate::{
    ast::{Location, identifier::Identifier},
    error::{FileId, Result, error},
};
use codespan_reporting::diagnostic::Diagnostic;
use std::ops::Range;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Source {
    file_id: FileId,
    range: (usize, usize),
}

impl Source {
    pub fn new(file_id: FileId, range: Range<usize>) -> Source {
        Source {
            file_id,
            range: (range.start, range.end),
        }
    }

    pub fn merge(&self, with: Source) -> Source {
        Source {
            file_id: self.file_id,
            range: (self.range.0, with.range.1),
        }
    }

    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    pub fn range(&self) -> Range<usize> {
        self.range.0..self.range.1
    }
}

impl TryFrom<&Vec<Identifier>> for Source {
    type Error = crate::error::Error;

    fn try_from(value: &Vec<Identifier>) -> Result<Source> {
        if value.is_empty() {
            return error(Diagnostic::error().with_message("expected that type path is not empty"));
        }
        Ok(value
            .first()
            .expect("expect first identifier exists")
            .merge(value.last().expect("expect last identifier exists")))
    }
}
