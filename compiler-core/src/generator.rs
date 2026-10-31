use crate::{ast::TypedDecl, error::Error};
use std::path::{Path, PathBuf};

pub fn output(path: &Path, folder: &Path, extension: &str) -> PathBuf {
    let mut path_buf = PathBuf::from(folder);
    path_buf.push(path);
    path_buf.set_extension(extension);
    return path_buf;
}

pub trait Generator<'a> {
    fn generate(&mut self, ast: &'a [TypedDecl], output: &Path) -> Result<(), Error>;
}

pub struct Compiler<'a> {
    ast: &'a [TypedDecl],
}

impl<'a> Compiler<'a> {
    pub fn new(ast: &'a [TypedDecl]) -> Compiler<'a> {
        Compiler { ast }
    }

    pub fn compile<T: Generator<'a>>(&self, mut generator: T, output: &Path) -> Result<(), Error> {
        generator.generate(self.ast, output)
    }
}
