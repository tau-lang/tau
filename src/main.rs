#![allow(clippy::needless_return)]
use crate::{
    ast::StmtVisitor,
    compiler::{
        Compiler,
        cpp::{CppCodeGenerator, CppHeaderGenerator},
        replace_extension,
    },
    header::Header,
    lexer::Lexer,
    parser::Parser,
    resolution::Resolution,
};
use std::{collections::HashMap, env, fs, io, path::PathBuf, rc::Rc, str::FromStr};

mod ast;
mod compiler;
mod error;
mod header;
mod lexer;
mod parser;
mod resolution;
mod typing;

fn main() -> error::Result<()> {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        1 => {
            let mut buffer = String::new();
            while io::stdin().read_line(&mut buffer).is_ok() {
                let lexer = Lexer::new(buffer.chars(), Rc::new(PathBuf::new()));
                let tokens = lexer.scan()?;
                // Check if user pressed ^D
                if tokens.len() > 1 {
                    let mut parser = Parser::new(tokens, &buffer);
                    let ast = parser.stmt()?;
                    let (types, fields) = (HashMap::new(), HashMap::new());
                    let mut resolution = Resolution::new(&types, fields);
                    resolution.visit_stmt(&ast);
                    println!("{:#?}", resolution);
                    buffer.clear();
                } else {
                    break;
                }
            }
        }
        2 => {
            let filename = args.get(1).unwrap();
            let content = fs::read_to_string(filename).expect("Expected to open file");
            let lexer = Lexer::new(
                content.chars(),
                Rc::new(PathBuf::from_str(filename).unwrap()),
            );
            let parser = Parser::new(lexer.scan()?, &content);
            let ast = parser.parse()?;
            let header = Header::new().headers(&ast);
            let (types, fields) = header.analysed();
            let resolution = Resolution::new(&types, fields).resolve(&ast);
            let _ = resolution.analysed();
            let compiler = Compiler::new(&ast);
            let header_output = replace_extension(filename, "hpp");
            compiler.compile(CppHeaderGenerator, &header_output)?;
            let code_output = replace_extension(filename, "cpp");
            compiler.compile(CppCodeGenerator::new(), &code_output)?;
        }
        _ => {
            // TODO:
            panic!("usage: {:?} [file]", args.first().unwrap());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::StmtVisitor, header::Header, lexer::Lexer, parser::Parser, resolution::Resolution,
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
        let mut parser = Parser::new(lexer.scan().unwrap(), content);
        println!("{:?}", parser.expr());
    }

    #[test]
    fn parse_stmt() {
        let content = "{ let a = 1 if (a < 2) break }";
        let lexer = Lexer::new(content.chars(), Rc::new(PathBuf::new()));
        let mut parser = Parser::new(lexer.scan().unwrap(), content);
        println!("{:?}", parser.stmt());
    }

    #[test]
    fn header_file() {
        let content = fs::read_to_string("examples/vec2.tau").expect("Expected to open file");
        let lexer = Lexer::new(content.chars(), Rc::new(PathBuf::new()));
        let parser = Parser::new(lexer.scan().unwrap(), &content);
        let ast = parser.parse().unwrap();
        let header = Header::new();
        println!("{:?}", header.headers(&ast).analysed());
    }

    #[test]
    fn resolve_stmt() {
        let content = "{ let a = 2 if (a < 2) break }";
        let lexer = Lexer::new(content.chars(), Rc::new(PathBuf::new()));
        let mut parser = Parser::new(lexer.scan().unwrap(), content);
        let ast = parser.stmt().unwrap();
        let (types, fields) = Header::new().analysed();
        let mut resolution = Resolution::new(&types, fields);
        println!("{:#?}", resolution.visit_stmt(&ast));
    }
}
