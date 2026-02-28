use crate::{ast::declaration::Decl, error::Error};
use std::path::{Path, PathBuf};

pub mod cpp;

pub fn set_output(path: &str, folder: &str, extension: &str) -> PathBuf {
    let mut path_buf = if folder != "" {
        let mut path_buf = PathBuf::from(folder);
        path_buf.push(PathBuf::from(path).file_name().expect("path is a file"));
        path_buf
    } else {
        PathBuf::from(path)
    };
    path_buf.set_extension(extension);
    return path_buf;
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
