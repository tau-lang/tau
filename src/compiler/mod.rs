use crate::ast::Decl;
use std::{
    io::Error,
    path::{Path, PathBuf},
};

pub mod cpp;

pub fn replace_extension(path: &str, new_ext: &str) -> PathBuf {
    let mut path_buf = PathBuf::from(path);
    path_buf.set_extension(new_ext);
    path_buf
}

pub trait Generator<'a> {
    fn generate(&mut self, ast: &'a [Decl], output: &Path) -> Result<(), Error>;
}

pub struct Compiler<'a> {
    ast: &'a [Decl],
}

impl<'a> Compiler<'a> {
    pub fn new(ast: &'a [Decl]) -> Compiler<'a> {
        Compiler { ast }
    }

    pub fn compile<T: Generator<'a>>(&self, mut generator: T, output: &Path) -> Result<(), Error> {
        generator.generate(self.ast, output)
    }
}
