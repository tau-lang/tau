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
use std::{fs, path::PathBuf, process::exit, rc::Rc};

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
    let args = ArgsBuilder::new().parse().unwrap().build();
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
    use crate::{
        ast::statement::StmtVisitor, header::Header, lexer::Lexer, parser::Parser,
        resolution::Resolution,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::rc::Rc;

    #[test]
    fn lexer() {
        let content = fs::read_to_string("examples/vec2.tau").expect("Expected to open file");
        let lexer = Lexer::new(content.chars(), Rc::new(PathBuf::new()));
        println!("{:?}", lexer.scan());
    }

    #[test]
    fn parse_expr() {
        let content = "(1+a[0]) * hypo(3, 4)";
        let lexer = Lexer::new(content.chars(), Rc::new(PathBuf::new()));
        let mut parser = Parser::new(lexer.scan().unwrap());
        println!("{:?}", parser.expr());
    }

    #[test]
    fn parse_stmt() {
        let content = "{ let a = 1 if (a < 2) break }";
        let lexer = Lexer::new(content.chars(), Rc::new(PathBuf::new()));
        let mut parser = Parser::new(lexer.scan().unwrap());
        println!("{:?}", parser.stmt());
    }

    #[test]
    fn header_file() {
        let content = fs::read_to_string("examples/vec2.tau").expect("Expected to open file");
        let lexer = Lexer::new(content.chars(), Rc::new(PathBuf::new()));
        let parser = Parser::new(lexer.scan().unwrap());
        let ast = parser.parse().unwrap();
        let header = Header::new();
        println!("{:?}", header.headers(&ast).analysed());
    }

    #[test]
    fn resolve_stmt() {
        let content = "{ let a = 2 if (a < 2) break }";
        let lexer = Lexer::new(content.chars(), Rc::new(PathBuf::new()));
        let mut parser = Parser::new(lexer.scan().unwrap());
        let ast = parser.stmt().unwrap();
        let (types, fields) = Header::new().analysed();
        let mut resolution = Resolution::new(&types, fields);
        println!("{:#?}", resolution.visit_stmt(&ast));
    }
}
