use crate::{
    ast::declaration::Decl,
    error::{FileId, Result},
    header::{Header, TypeMap},
    lexer::Lexer,
    parser::Parser,
};

pub mod ast;
pub mod error;
pub mod generator;
pub mod header;
pub mod lexer;
pub mod parser;
pub mod resolution;
pub mod type_;

pub fn process_file(file_id: FileId, content: &str) -> Result<(Vec<Decl<()>>, TypeMap, TypeMap)> {
    let lexer = Lexer::new(&content, file_id);
    let parser = Parser::new(lexer);
    let ast = parser.parse()?;
    let header = Header::new().headers(&ast)?;
    let (types, fields) = header.analysed();
    // let fields = Resolution::new(&types, fields).resolve(&ast)?;
    Ok((ast, types, fields))
}
