//! Reads the tokens from an input

use crate::error::{Error, FileId, Result, diagnostic, error};
use std::str::CharIndices;

pub mod source;
pub mod token;

pub use source::Source;
pub use token::{Token, TokenType};

/// The lexer reads a source string slice and converts it to tokens. This struct
/// is also an iterator, that reads the next token whenever next is called. The
/// token scanner always assumes that the internal byte offset points to the
/// start of the next token.
#[derive(Clone)]
pub struct Lexer<'a> {
    source: &'a str,
    chars: CharIndices<'a>,
    file_id: FileId,
    start: usize,
    last: Option<(usize, char)>,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer with a given source and file id.
    /// The file id is used to track from which file the source originates. If
    /// later an error occurs, this is the id that is used for codespan.
    pub fn new(source: &'a str, file_id: FileId) -> Lexer<'a> {
        let mut lexer = Lexer {
            source,
            chars: source.char_indices(),
            file_id,
            start: 0,
            last: None,
        };
        // We need to skip leading spaces so that the lexer directly starts at
        // the first token.
        lexer.skip_space();
        lexer
    }

    /// Scans the input for the next token. Throws an error if it could not
    /// parse a token. This may happen because a string literal was not
    /// terminated or a float literal contained two dots.
    fn scan_token(&mut self) -> Result<Token> {
        self.start = self.end();
        let c = self
            .advance()
            .ok_or_else(|| Error::new(diagnostic("expected token, found eof", self.source())))?;
        let token_type = match c {
            '(' => TokenType::ParenLeft,
            ')' => TokenType::ParenRight,
            '{' => TokenType::BraceLeft,
            '}' => TokenType::BraceRight,
            '[' => TokenType::BracketLeft,
            ']' => TokenType::BracketRight,
            ',' => TokenType::Comma,
            '.' => TokenType::Dot,
            ':' => {
                if self.matchc(':') {
                    TokenType::Access
                } else {
                    TokenType::Colon
                }
            }
            ';' => TokenType::Semicolon,
            '+' => {
                if self.matchc('=') {
                    TokenType::SetAdd
                } else {
                    TokenType::Add
                }
            }
            '-' => {
                if self.matchc('=') {
                    TokenType::SetSub
                } else if self.matchc('>') {
                    TokenType::To
                } else {
                    TokenType::Sub
                }
            }
            '*' => {
                if self.matchc('=') {
                    TokenType::SetMul
                } else {
                    TokenType::Star
                }
            }
            '/' => {
                if self.matchc('/') {
                    while self.is_non_empty() && self.peek().unwrap() != '\n' {
                        self.advance();
                    }
                    return self.scan_token();
                } else if self.matchc('=') {
                    TokenType::SetDiv
                } else {
                    TokenType::Div
                }
            }
            '%' => TokenType::Mod,
            '&' => {
                if self.matchc('&') {
                    TokenType::And
                } else {
                    TokenType::Ref
                }
            }
            '|' => {
                if self.matchc('|') {
                    TokenType::Or
                } else {
                    TokenType::Vbar
                }
            }
            '=' => {
                if self.matchc('=') {
                    TokenType::Eq
                } else {
                    TokenType::Set
                }
            }
            '!' => {
                if self.matchc('=') {
                    TokenType::Neq
                } else {
                    TokenType::Not
                }
            }
            '>' => {
                if self.matchc('=') {
                    TokenType::Geq
                } else {
                    TokenType::Gre
                }
            }
            '<' => {
                if self.matchc('=') {
                    TokenType::Leq
                } else {
                    TokenType::Low
                }
            }
            '"' => self.string()?,
            _ => {
                if Lexer::is_digit(c) {
                    self.number()
                } else if Lexer::is_alpha(c) {
                    self.identifier()
                } else {
                    return error(diagnostic(
                        format!("unexpected character '{}'", c),
                        self.source(),
                    ));
                }
            }
        };
        let source = self.source();
        // When we call this method again, we expect that is points again at the
        // next non-space character.
        self.skip_space();
        Ok(Token::new(token_type, source))
    }

    fn identifier(&mut self) -> TokenType {
        self.start = self.chars.offset() - 1;
        while self.is_non_empty() && Lexer::is_alpha_numeric(self.peek().unwrap()) {
            self.advance();
        }

        let text = self.view();
        match text {
            "import" => TokenType::Import,
            "struct" => TokenType::Struct,
            "io" => TokenType::Io,
            "in" => TokenType::In,
            "fn" => TokenType::Function,
            "const" => TokenType::Const,
            "extern" => TokenType::Extern,
            "let" => TokenType::Let,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "for" => TokenType::For,
            "while" => TokenType::While,
            "return" => TokenType::Return,
            "break" => TokenType::Break,
            "self" => TokenType::VSelf,
            "true" => TokenType::Bool(true),
            "false" => TokenType::Bool(false),
            _ => TokenType::Identifier(text.into()),
        }
    }

    fn number(&mut self) -> TokenType {
        self.start = self.chars.offset() - 1;
        while self.is_non_empty() {
            let next = self.peek().unwrap();
            if Lexer::is_digit(next) {
                self.advance();
            } else if next == '.' {
                self.advance();
                return self.float();
            } else {
                break;
            }
        }

        TokenType::Int(self.view().parse().unwrap())
    }

    fn float(&mut self) -> TokenType {
        while self.is_non_empty() {
            let next = self.peek().unwrap();
            if Lexer::is_digit(next) {
                self.advance();
            } else {
                break;
            }
        }

        TokenType::Float(self.view().parse().unwrap())
    }

    /// Parses a single string.
    fn string(&mut self) -> Result<TokenType> {
        self.start = self.chars.offset();
        while self.is_non_empty() && self.peek().unwrap() != '"' {
            self.advance();
        }

        let view = self.view().to_string();
        // The enclosing ".
        self.advance()
            .ok_or_else(|| Error::new(diagnostic("unclosed string literal", self.source())))?;

        Ok(TokenType::String(view))
    }

    fn matchc(&mut self, c: char) -> bool {
        match self.peek() {
            Some(p) => p == c && self.advance().is_some(),
            _ => false,
        }
    }

    fn peek(&mut self) -> Option<char> {
        if let Some((_, c)) = self.last {
            return Some(c);
        }
        let o = self.chars.offset();
        let c = self.advance()?;
        self.last = Some((o, c));
        Some(c)
    }

    fn skip_space(&mut self) {
        while let Some(c) = self.peek()
            && matches!(c, ' ' | '\r' | '\t' | '\n')
        {
            self.advance().expect("expected token exists");
        }
    }

    fn is_alpha(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_'
    }

    fn is_digit(c: char) -> bool {
        c.is_ascii_digit()
    }

    fn is_alpha_numeric(c: char) -> bool {
        Lexer::is_alpha(c) || Lexer::is_digit(c)
    }

    fn is_non_empty(&mut self) -> bool {
        self.peek().is_some()
    }

    fn advance(&mut self) -> Option<char> {
        if let Some((_, c)) = self.last.take() {
            return Some(c);
        }
        self.chars.next().map(|v| v.1)
    }

    fn end(&self) -> usize {
        if let Some((o, _)) = &self.last {
            *o
        } else {
            self.chars.offset()
        }
    }

    fn view(&self) -> &str {
        &self.source[self.start..self.end()]
    }

    fn source(&self) -> Source {
        Source::new(self.file_id, self.start..self.end())
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_non_empty() {
            Some(self.scan_token())
        } else {
            None
        }
    }
}
