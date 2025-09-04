use crate::{lexer::Lexer, parser::Parser};
use std::{env, fs, io};

mod ast;
mod lexer;
mod parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        1 => {
            let mut buffer = String::new();
            while io::stdin().read_line(&mut buffer).is_ok() {
                let lexer = Lexer::new(buffer.chars());
                let tokens = lexer.scan();
                if tokens.len() > 1 {
                    let mut parser = Parser::new(tokens);
                    println!("{:#?}", parser.stmt());
                    buffer.clear();
                } else {
                    break;
                }
            }
        }
        2 => {
            let content = fs::read_to_string(args.get(1).unwrap()).expect("Expected to open file");
            let lexer = Lexer::new(content.chars());
            let parser = Parser::new(lexer.scan());
            println!("{:#?}", parser.parse());
        }
        _ => println!("usage: {:?} [file]", args.first().unwrap()),
    }
}

#[cfg(test)]
mod tests {
    use crate::{lexer::Lexer, parser::Parser};
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
}
