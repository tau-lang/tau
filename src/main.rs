use crate::{
    ast::StmtVisitor,
    compiler::{
        Compiler,
        c::{CCodeGenerator, CHeaderGenerator},
        replace_extension,
    },
    header::Header,
    lexer::Lexer,
    parser::Parser,
    resolution::Resolution,
};
use std::{env, fs, io};

mod ast;
mod compiler;
mod header;
mod lexer;
mod parser;
mod resolution;

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        1 => {
            let mut buffer = String::new();
            while io::stdin().read_line(&mut buffer).is_ok() {
                let lexer = Lexer::new(buffer.chars());
                let tokens = lexer.scan();
                // Check if user pressed ^D
                if tokens.len() > 1 {
                    let mut parser = Parser::new(tokens);
                    let ast = parser.stmt();
                    let mut resolution = Resolution::new(Header::new());
                    resolution.visit_stmt(&ast);
                    println!("{:#?}", resolution);
                    buffer.clear();
                } else {
                    break;
                }
            }
            Ok(())
        }
        2 => {
            let filename = args.get(1).unwrap();
            let content = fs::read_to_string(filename).expect("Expected to open file");
            let lexer = Lexer::new(content.chars());
            let parser = Parser::new(lexer.scan());
            let ast = parser.parse();
            let header = Header::new().headers(&ast);
            let resolution = Resolution::new(header);
            let _scopes = resolution.resolve(&ast).analysed();
            let compiler = Compiler::new(ast);
            let mut header_output = fs::File::create(replace_extension(filename, "h"))?;
            compiler.compile(&mut CHeaderGenerator, &mut header_output)?;
            let mut code_output = fs::File::create(replace_extension(filename, "c"))?;
            compiler.compile(&mut CCodeGenerator, &mut code_output)?;
            Ok(())
        }
        _ => {
            println!("usage: {:?} [file]", args.first().unwrap());
            Err(io::Error::new(io::ErrorKind::Other, "false usage"))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::StmtVisitor, header::Header, lexer::Lexer, parser::Parser, resolution::Resolution,
    };
    use std::fs;

    #[test]
    fn lexer() {
        let content = fs::read_to_string("examples/vec2.tau").expect("Expected to open file");
        let lexer = Lexer::new(content.chars());
        println!("{:?}", lexer.scan());
    }

    #[test]
    fn parse_expr() {
        let content = "(1+a[0]) * hypo(3, 4)";
        let lexer = Lexer::new(content.chars());
        let mut parser = Parser::new(lexer.scan());
        println!("{:?}", parser.expr());
    }

    #[test]
    fn parse_stmt() {
        let content = "{ let a = 1 if (a < 2) break }";
        let lexer = Lexer::new(content.chars());
        let mut parser = Parser::new(lexer.scan());
        println!("{:?}", parser.stmt());
    }

    #[test]
    fn header_file() {
        let content = fs::read_to_string("examples/vec2.tau").expect("Expected to open file");
        let lexer = Lexer::new(content.chars());
        let parser = Parser::new(lexer.scan());
        let ast = parser.parse();
        let header = Header::new();
        println!("{:?}", header.headers(&ast).analysed());
    }

    #[test]
    fn resolve_stmt() {
        let content = "{ let a = 2 if (a < 2) break }";
        let lexer = Lexer::new(content.chars());
        let mut parser = Parser::new(lexer.scan());
        let ast = parser.stmt();
        let mut resolution = Resolution::new(Header::new());
        println!("{:#?}", resolution.visit_stmt(&ast));
    }
}
