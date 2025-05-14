pub mod ast;
pub mod parser;

use pest::Parser;

fn main() {
    let pairs = parser::TauParser::parse(parser::Rule::root, include_str!("../examples/vec2.tau"))
        .unwrap_or_else(|e| panic!("{}", e));
    dbg!(
        pairs
            .map(|pair| parser::parse_tau(pair))
            .collect::<Vec<ast::Ast>>()
    );
}
