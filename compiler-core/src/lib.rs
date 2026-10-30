use crate::{
    analyser::SemanticAnalyser,
    ast::TypedDecl,
    context::Context,
    error::{FileId, Result},
    header::Header,
    lexer::Lexer,
    parser::Parser,
    type_::environment::Environment,
};

pub mod analyser;
pub mod ast;
pub mod context;
pub mod error;
pub mod generator;
pub mod header;
pub mod lexer;
pub mod parser;
pub mod type_;
pub mod uid;

pub fn process_file<'ctx>(
    context: &'ctx mut Context,
    file_id: FileId,
    content: &str,
) -> Result<(Vec<TypedDecl>, Environment)> {
    let lexer = Lexer::new(&content, file_id);
    let ast = Parser::new(lexer).parse()?;
    let env = Header::new(context).env(&ast)?;
    let mut analyser = SemanticAnalyser::new(env);
    let ast = analyser.fold_ast(ast)?;
    let env = analyser.env();
    Ok((ast, env))
}
