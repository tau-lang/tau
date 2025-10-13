pub mod c_transpiler;
pub mod crane;

use crate::ast::Decl;
use std::fs::File;
use std::io::Error;

pub trait Generator {
    fn generate(&mut self, ast: &[Decl], output: &mut File) -> Result<(), Error>;
}

pub struct Compiler {
    ast: Vec<Decl>,
}

impl Compiler {
    pub fn new(ast: Vec<Decl>) -> Compiler {
        Compiler { ast }
    }

    pub fn compile<T: Generator>(&self, generator: &mut T, output: &mut File) -> Result<(), Error> {
        generator.generate(&self.ast, output)
    }
}
