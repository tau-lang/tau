pub mod ast;
pub mod parser;

use parser::{Parser, TauParser};

fn main() {
    let _ = dbg!(TauParser::parse(include_str!("../examples/vec2.tau")));
    // let pairs = parser::TauParser::parse(parser::Rule::root, include_str!("../examples/vec2.tau"))
    //     .unwrap_or_else(|e| panic!("{}", e));
    // dbg!(
    //     pairs
    //         .map(|pair| parser::parse_tau(pair))
    //         .collect::<Vec<ast::Ast>>()
    // );
}
