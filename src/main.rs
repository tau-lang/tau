#![allow(clippy::needless_return)]
use crate::{
    cli::{ArgsBuilder, HELP_MESSAGE},
    compiler::{
        Compiler,
        cpp::{CppCodeGenerator, CppHeaderGenerator},
        set_output,
    },
    header::Header,
    lexer::Lexer,
    parser::Parser,
    resolution::Resolution,
};
use std::{env, fs, path::PathBuf, process::exit, rc::Rc};

mod ast;
mod cli;
mod compiler;
mod error;
mod header;
mod lexer;
mod parser;
mod resolution;
mod typing;

fn main() -> error::Result<()> {
    let args = ArgsBuilder::new()
        .parse(env::args().collect())
        .unwrap()
        .build();
    if args.input().is_empty() {
        println!("{}", HELP_MESSAGE)
    } else {
        // guaranteed to not be empty
        for filename in args.input() {
            compile_file(filename, &args)?;
        }
    }
    Ok(())
}

fn compile_file(filename: &Rc<PathBuf>, args: &cli::Args) -> error::Result<()> {
    if let Ok(content) = fs::read_to_string(filename.as_path()) {
        let lexer = Lexer::new(content.chars(), filename.clone());
        let parser = Parser::new(lexer.scan()?);
        let ast = parser.parse()?;
        let header = Header::new().headers(&ast);
        let (types, fields) = header.analysed();
        Resolution::new(&types, fields).resolve(&ast)?;
        match args.target() {
            cli::Target::Cpp => {
                let compiler = Compiler::new(&ast);
                let header_output = set_output(filename.as_ref(), args.output(), "hpp");
                compiler.compile(CppHeaderGenerator, &header_output)?;
                let code_output = set_output(filename.as_ref(), args.output(), "cpp");
                compiler.compile(CppCodeGenerator::new(), &code_output)?;
            }
            cli::Target::Cranelift => todo!("hahaha i wish"),
        }
        Ok(())
    } else {
        println!("tau: cannot access '{}: No such file", filename.display());
        exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::compile_file;
    use crate::{cli::ArgsBuilder, error, lexer::Lexer, parser::Parser};
    use std::{path::PathBuf, rc::Rc};

    #[test]
    fn pipeline_arrays() -> error::Result<()> {
        let args = ArgsBuilder::new().build();
        compile_file(&Rc::new(PathBuf::from("examples/arrays.tau")), &args)
    }

    #[test]
    fn pipeline_heap() -> error::Result<()> {
        let args = ArgsBuilder::new().build();
        compile_file(&Rc::new(PathBuf::from("examples/heap.tau")), &args)
    }

    #[test]
    fn pipeline_vec2() -> error::Result<()> {
        let args = ArgsBuilder::new().build();
        compile_file(&Rc::new(PathBuf::from("examples/vec2.tau")), &args)
    }

    #[test]
    fn infinite_scoping() -> error::Result<()> {
        let content = "{ { { { { { {} } } } } } }";
        let lexer = Lexer::new(content.chars(), Rc::new(PathBuf::new()));
        let mut parser = Parser::new(lexer.scan().unwrap());
        parser.stmt().map(|_| {})
    }
}
