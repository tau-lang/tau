#![deny(clippy::unwrap_used)]

pub mod ast;
pub mod parser;

use parser::{Parser, TauParser};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ast: ast::Ast = TauParser::parse(include_str!("../examples/vec2.tau"))?;
    // dbg!(ast.check_types())?;
    Ok(())
}
