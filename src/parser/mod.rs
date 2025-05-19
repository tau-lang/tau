pub mod error;
mod parser;
pub(crate) mod typechecked;

use crate::ast;
use error::ParserError;
use miette::SourceOffset;
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
        let mut lexed = TauLexer::parse(Rule::root, src_code).map_err(|e| {
            dbg!(&e);
            let (lin, col): (usize, usize) = match e.line_col {
                pest::error::LineColLocation::Pos((lin, col)) => (lin, col),
                pest::error::LineColLocation::Span((lin, col), _) => (lin, col),
            };
            ParserError::Lexer(error::LexError {
                cause: match e.variant {
                    #[allow(unused_variables)]
                    pest::error::ErrorVariant::ParsingError {
                        positives,
                        negatives,
                    } => error::LexErrorVarient::Parsing(positives.first().unwrap().clone()),
                    pest::error::ErrorVariant::CustomError { message } => {
                        error::LexErrorVarient::Custom(message)
                    }
                },
                input: src_code.to_string(),
                location: SourceOffset::from_location(src_code, lin, col),
            })
        })?;
        parser::parse_tau(lexed.next().ok_or(error::ParserError::EmptyInput)?)
            .map_err(error::ParserError::Parser)
    }
}
