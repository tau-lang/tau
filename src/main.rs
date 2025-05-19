#![deny(clippy::unwrap_used)]

pub mod ast;
pub mod parser;

use parser::{Parser, TauParser};

fn main() -> miette::Result<()> {
    let ast: ast::Ast = dbg!(TauParser::parse(include_str!("../examples/vec2.tau"))?);
    Ok(())
}
