pub mod error;
mod parser;

use crate::ast;
use error::ParserError;
use pest::Parser as PParser;
use pest_derive::Parser as PParser;

#[derive(PParser)]
#[grammar = "../grammar/tau.pest"]
struct TauLexer;

pub trait Parser {
    fn parse(src_code: &str) -> Result<ast::Ast, ParserError>;
}

pub struct TauParser;

impl Parser for TauParser {
    fn parse(src_code: &str) -> Result<ast::Ast, ParserError> {
        let mut lexed = TauLexer::parse(Rule::root, src_code).map_err(ParserError::Lexer)?;
        Ok(
            parser::parse_tau(lexed.next().ok_or(error::ParserError::EmptyInput)?)
                .map_err(error::ParserError::Parser)?,
        )
    }
}
