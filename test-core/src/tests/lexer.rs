use crate::assert_ok;
use symbol_table::static_symbol;
use tau_core::{
    error::Result,
    lexer::{Lexer, Token, TokenType},
};

#[test]
fn lexes_utf8_str() {
    let file_id = 0;
    let lexer = Lexer::new(r#""Tau (τ) ⭐""#, file_id);
    let tokens = lexer.collect::<Vec<Result<Token>>>();
    let expected = vec![TokenType::String("Tau (τ) ⭐".to_string())];
    assert_eq!(tokens.len(), expected.len());
    for (token, expected) in tokens.iter().zip(expected) {
        assert_ok!(&token);
        assert_eq!(token.as_ref().unwrap().token_type(), &expected);
    }
}

#[test]
fn lexes_keywords() {
    let file_id = 0;
    let lexer = Lexer::new(
        r#"
    struct fn while if else let in "let"
  "#,
        file_id,
    );
    let tokens = lexer.collect::<Vec<Result<Token>>>();
    let expected = vec![
        TokenType::Struct,
        TokenType::Function,
        TokenType::While,
        TokenType::If,
        TokenType::Else,
        TokenType::Let,
        TokenType::In,
        TokenType::String("let".to_string()),
    ];
    assert_eq!(tokens.len(), expected.len());
    for (token, expected) in tokens.iter().zip(expected) {
        assert_ok!(&token);
        assert_eq!(token.as_ref().unwrap().token_type(), &expected);
    }
}

#[test]
fn lexes_numbers() {
    let file_id = 0;
    let lexer = Lexer::new(
        r#"
    1 1.0 0.001 1234
  "#,
        file_id,
    );
    let tokens = lexer.collect::<Vec<Result<Token>>>();
    let expected = vec![
        TokenType::Int(1),
        TokenType::Float(1.0),
        TokenType::Float(0.001),
        TokenType::Int(1234),
    ];
    assert_eq!(tokens.len(), expected.len());
    for (token, expected) in tokens.iter().zip(expected) {
        assert_ok!(&token);
        assert_eq!(token.as_ref().unwrap().token_type(), &expected);
    }
}

#[test]
fn lexes_identifiers() {
    let file_id = 0;
    let lexer = Lexer::new(
        r#"
    a this_is_a_VERY_long_Identifier while
  "#,
        file_id,
    );
    let tokens = lexer.collect::<Vec<Result<Token>>>();
    let expected = vec![
        TokenType::Identifier(static_symbol!("a")),
        // Identifier with underscores.
        TokenType::Identifier(static_symbol!("this_is_a_VERY_long_Identifier")),
        // Keywords are still lexed first.
        TokenType::While,
    ];
    assert_eq!(tokens.len(), expected.len());
    for (token, expected) in tokens.iter().zip(expected) {
        assert_ok!(&token);
        assert_eq!(token.as_ref().unwrap().token_type(), &expected);
    }
}
