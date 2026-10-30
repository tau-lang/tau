use crate::error::{Error, FileId, Result, diagnostic, error};
use codespan_reporting::diagnostic::Diagnostic;
use std::str::CharIndices;

pub mod source;
pub mod token;

pub use source::Source;
pub use token::{Token, TokenType};

#[derive(Clone)]
pub struct Lexer<'a> {
    source: &'a str,
    chars: CharIndices<'a>,
    file_id: FileId,
    line: usize,
    start: usize,
    last: Option<(usize, char)>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, file_id: FileId) -> Lexer<'a> {
        Lexer {
            source: source,
            chars: source.char_indices(),
            file_id,
            line: 1,
            start: 0,
            last: None,
        }
    }

    fn scan_token(&mut self) -> Result<Token> {
        let c = self.advance()?;
        let token_type = match c {
            '(' => TokenType::ParenLeft,
            ')' => TokenType::ParenRight,
            '{' => TokenType::BraceLeft,
            '}' => TokenType::BraceRight,
            '[' => TokenType::BracketLeft,
            ']' => TokenType::BracketRight,
            ',' => TokenType::Comma,
            '.' => TokenType::Dot,
            ':' => TokenType::Colon,
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
                    while !self.is_at_end() && self.peek()? != '\n' {
                        self.advance()?;
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
            ' ' | '\r' | '\t' => {
                self.start = self.chars.offset();
                return self.scan_token();
            }
            '\n' => {
                self.line += 1;
                self.start = self.chars.offset();
                return self.scan_token();
            }
            '"' => self.string()?,
            _ => {
                if Lexer::is_digit(c) {
                    self.number()?
                } else if Lexer::is_alpha(c) {
                    self.identifier()?
                } else {
                    return error(diagnostic(
                        format!("unexpected character '{}'", c),
                        self.source(),
                    ));
                }
            }
        };
        Ok(Token::new(token_type, self.source()))
    }

    fn identifier(&mut self) -> Result<TokenType> {
        self.start = self.chars.offset() - 1;
        while !self.is_at_end() && Lexer::is_alpha_numeric(self.peek()?) {
            self.advance()?;
        }

        let text = self.view();
        Ok(match text {
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
        })
    }

    fn number(&mut self) -> Result<TokenType> {
        self.start = self.chars.offset() - 1;
        while !self.is_at_end() {
            let next = self.peek()?;
            if Lexer::is_digit(next) {
                self.advance()?;
            } else if next == '.' {
                self.advance()?;
                return self.float();
            } else {
                break;
            }
        }

        Ok(TokenType::Int(self.view().parse().unwrap()))
    }

    fn float(&mut self) -> Result<TokenType> {
        while !self.is_at_end() {
            let next = self.peek()?;
            if Lexer::is_digit(next) {
                self.advance()?;
            } else if next == '.' {
                return error(diagnostic("found a second dot in the float", self.source()));
            } else {
                break;
            }
        }

        Ok(TokenType::Float(self.view().parse().unwrap()))
    }

    /// Parses a single string.
    fn string(&mut self) -> crate::error::Result<TokenType> {
        self.start = self.chars.offset();
        while !self.is_at_end() && self.peek()? != '"' {
            let c = self.advance()?;
            if c == '\n' {
                self.line += 1;
            }
        }

        let view = self.view().to_string();
        // The enclosing ".
        self.advance()?;

        Ok(TokenType::String(view))
    }

    fn matchc(&mut self, c: char) -> bool {
        match self.peek() {
            Ok(p) => p == c && self.advance().is_ok(),
            _ => false,
        }
    }

    fn peek(&mut self) -> Result<char> {
        if let Some((_, c)) = self.last {
            return Ok(c);
        }
        let o = self.chars.offset();
        let c = self.advance()?;
        self.last = Some((o, c));
        Ok(c)
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

    fn is_at_end(&self) -> bool {
        self.source.len() <= self.chars.offset()
    }

    fn advance(&mut self) -> Result<char> {
        if let Some((_, c)) = self.last.take() {
            return Ok(c);
        }
        self.chars
            .next()
            .map(|v| v.1)
            .ok_or_else(|| Error::new(vec![Diagnostic::error().with_message("unexpected eof")]))
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
        if self.is_at_end() {
            None
        } else {
            Some(self.scan_token())
        }
    }
}
