use crate::{
    ast::{
        Location,
        identifier::Identifier,
        origin::{BinaryOp, ExprOrigin, Literal, UnaryOp},
    },
    error::{self, FileId},
    lexer::source::Source,
};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use std::fmt::Debug;
use symbol_table::{GlobalSymbol, static_symbol};

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum TokenType {
    // Brackets
    ParenLeft,    // (
    ParenRight,   // )
    BraceLeft,    // {
    BraceRight,   // }
    BracketLeft,  // [
    BracketRight, // ]

    // Punctuation
    Dot,
    Comma,     // ,
    Colon,     // :
    Semicolon, // ;
    To,        // ->

    // Operators
    Add,
    Sub,
    Div,
    Star,
    Set,
    SetAdd,
    SetSub,
    SetMul,
    SetDiv,
    Mod,
    Low,
    Gre,
    Geq,
    Leq,
    Eq,
    Neq,
    Not,
    And,
    Or,
    Vbar,
    Ref,

    // Keywords
    Import,
    Io,
    In,
    Function,
    Struct,
    Enum,
    If,
    Else,
    While,
    For,
    Match,
    Let,
    Const,
    Extern,
    Return,
    Break,
    VSelf,

    // Literal
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),

    // Identifiers, e.g types and variables
    Identifier(GlobalSymbol),

    // Misc
    Eof,
}

impl TokenType {
    pub fn is_identifier(&self) -> bool {
        matches!(self, Self::Identifier(_))
    }
}

impl TryFrom<TokenType> for BinaryOp {
    type Error = Diagnostic<FileId>;

    fn try_from(value: TokenType) -> Result<Self, Self::Error> {
        Ok(match value {
            TokenType::Add => BinaryOp::Add,
            TokenType::Sub => BinaryOp::Sub,
            TokenType::Div => BinaryOp::Div,
            TokenType::Star => BinaryOp::Mul,
            TokenType::Mod => BinaryOp::Mod,

            TokenType::Set => BinaryOp::Set,
            TokenType::SetAdd => BinaryOp::SetAdd,
            TokenType::SetSub => BinaryOp::SetSub,
            TokenType::SetMul => BinaryOp::SetMul,
            TokenType::SetDiv => BinaryOp::SetDiv,

            TokenType::Eq => BinaryOp::Eq,
            TokenType::Neq => BinaryOp::Neq,

            TokenType::Low => BinaryOp::Low,
            TokenType::Leq => BinaryOp::Leq,
            TokenType::Gre => BinaryOp::Gre,
            TokenType::Geq => BinaryOp::Geq,

            TokenType::And => BinaryOp::And,
            TokenType::Or => BinaryOp::Or,

            _ => return Err(Diagnostic::error().with_message("token is not a binary operator")),
        })
    }
}

impl TryFrom<TokenType> for UnaryOp {
    type Error = Diagnostic<FileId>;

    fn try_from(value: TokenType) -> Result<Self, Self::Error> {
        Ok(match value {
            TokenType::Sub => Self::Neg,
            TokenType::Not => Self::Not,
            TokenType::Ref => Self::Ref,
            TokenType::Star => Self::Deref,

            _ => {
                return Err(Diagnostic::error().with_message("token is not a unary operator"));
            }
        })
    }
}

impl TryFrom<TokenType> for Literal {
    type Error = Diagnostic<FileId>;

    fn try_from(value: TokenType) -> Result<Self, Self::Error> {
        Ok(match value {
            TokenType::Bool(value) => Self::Bool(value),
            TokenType::Int(value) => Self::Int(value),
            TokenType::Float(value) => Self::Float(value),
            TokenType::String(value) => Self::String(value),

            _ => {
                return Err(Diagnostic::error().with_message("token is not a literal"));
            }
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    token_type: TokenType,
    source: Source,
}

impl Token {
    pub fn new(token_type: TokenType, source: Source) -> Token {
        Token { token_type, source }
    }

    pub fn token_type(&self) -> &TokenType {
        &self.token_type
    }
}

impl Location for Token {
    fn source(&self) -> Source {
        self.source
    }
}

impl From<Token> for TokenType {
    fn from(value: Token) -> Self {
        value.token_type
    }
}

impl From<Token> for Source {
    fn from(value: Token) -> Self {
        value.source
    }
}

impl<O> TryFrom<Token> for ExprOrigin<O>
where
    O: TryFrom<TokenType, Error = Diagnostic<FileId>> + Debug,
{
    type Error = error::Error;

    fn try_from(value: Token) -> error::Result<Self> {
        let (type_, source) = (value.token_type, value.source);
        let kind = O::try_from(type_).map_err(|e| {
            error::Error::new(vec![
                e.with_label(Label::primary(source.file_id(), source.range())),
            ])
        })?;
        Ok(Self::new(kind, source))
    }
}

impl From<Token> for Identifier {
    fn from(token: Token) -> Self {
        match token.token_type() {
            TokenType::Identifier(name) => Identifier::new(*name, token.source),
            TokenType::VSelf => Identifier::new(static_symbol!("self"), token.source),
            // TODO: Maybe replace with TryFrom?
            _ => panic!("token '{:?}' is not a identifier", token),
        }
    }
}

impl From<&Token> for Identifier {
    fn from(token: &Token) -> Self {
        if let TokenType::Identifier(name) = token.token_type() {
            Identifier::new(*name, token.source.clone())
        } else {
            // TODO: Maybe replace with TryFrom?
            panic!("can only be used on an identifier")
        }
    }
}
