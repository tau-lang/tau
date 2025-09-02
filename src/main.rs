mod lexer;

use crate::lexer::Lexer;
use std::fs;

fn main() {
    let content = fs::read_to_string("examples/vec2.tau").expect("Expected to open file");
    let lexer = Lexer::new(content.chars());
    print!("{:?}", lexer.scan());
}
