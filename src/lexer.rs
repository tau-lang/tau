use crate::{
    ast::identifier::Identifier,
    error::{Diagnostic, Error, Result, lexer_expected},
};
use std::{
    collections::VecDeque,
    fmt::{Display, Formatter},
    fs, io,
    iter::Peekable,
    path::PathBuf,
    rc::Rc,
    str::Chars,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    file: Rc<PathBuf>,
    line: usize,
    start: usize,
    end: usize,
}

#[cfg(test)]
impl Default for Source {
    fn default() -> Self {
        Source {
            file: Rc::new(PathBuf::new()),
            line: 0,
            start: 0,
            end: 0,
        }
    }
}

impl Source {
    pub fn new(file: Rc<PathBuf>, line: usize, start: usize, end: usize) -> Source {
        Source {
            file,
            line,
            start,
            end,
        }
    }

    pub fn union(left: &Source, right: &Source) -> Source {
        Source {
            file: left.file.clone(),
            line: left.line,
            start: left.start,
            end: right.end,
        }
    }

    pub fn content(&self) -> io::Result<String> {
        let content = fs::read_to_string(self.file.to_path_buf())?;

        return Ok(content[self.start..self.end].to_string());
    }
}

impl Display for Source {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(formatter, "{}:{}", self.file.to_string_lossy(), self.line)
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

    pub fn get_type(&self) -> &TokenType {
        &self.token_type
    }

    pub fn identifier(&self) -> &str {
        match self.get_type() {
            TokenType::Identifier(name) => name,
            TokenType::VSelf => "self",
            _ => panic!("expected identifier"),
        }
    }
    pub fn get_source(&self) -> Source {
        self.source.clone()
    }
}

impl From<Token> for Identifier {
    fn from(token: Token) -> Self {
        match token.get_type() {
            TokenType::Identifier(name) => Identifier::new(name.to_string(), token.source),
            TokenType::VSelf => Identifier::new("self".to_string(), token.source),
            _ => panic!("token '{:?}' is not a identifier", token),
        }
    }
}

impl From<&Token> for Identifier {
    fn from(token: &Token) -> Self {
        if let TokenType::Identifier(name) = token.get_type() {
            Identifier::new(name.to_string(), token.source.clone())
        } else {
            panic!()
        }
    }
}

impl From<Token> for TokenType {
    fn from(value: Token) -> Self {
        value.token_type
    }
}

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
    Dot,
    Comma, // ,
    Colon, // :
    To,    // ->

    // Operators
    Add,
    Sub,
    Div,
    Mul,
    Set,
    SetAdd,
    SetSub,
    SetMul,
    SetDiv,
    Low,
    Gre,
    Geq,
    Leq,
    Eq,
    Neq,
    Not,
    And,
    Or,
    Xor,

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
    Number(f64),
    String(String),
    Identifier(String),

    // Misc
    Eof,
}

impl TokenType {
    pub fn is_identifer(&self) -> bool {
        matches!(self, Self::Identifier(_))
    }
}

#[derive(Clone)]
pub struct Lexer<'a> {
    source: Peekable<Chars<'a>>,
    tokens: VecDeque<Token>,
    file: Rc<PathBuf>,
    line: usize,
    start: usize,
    offset: usize,
}

impl Lexer<'_> {
    pub fn new(source: Chars<'_>, file: Rc<PathBuf>) -> Lexer<'_> {
        Lexer {
            source: source.peekable(),
            file,
            tokens: VecDeque::new(),
            line: 1,
            start: 0,
            offset: 0,
        }
    }

    pub fn scan(mut self) -> Result<VecDeque<Token>> {
        let mut error = Vec::new();
        while !self.is_at_end() {
            if let Err(e) = self.scan_token() {
                error.push(e);
            }
        }
        if !error.is_empty() {
            Err(error.into_iter().reduce(|a, b| a.concat(b)).unwrap())
        } else {
            Ok(self.tokens)
        }
    }

    fn scan_token(&mut self) -> Result<()> {
        let token_type = if let Some(c) = self.advance() {
            match c {
                '(' => TokenType::ParenLeft,
                ')' => TokenType::ParenRight,
                '{' => TokenType::BraceLeft,
                '}' => TokenType::BraceRight,
                '[' => TokenType::BracketLeft,
                ']' => TokenType::BracketRight,
                ',' => TokenType::Comma,
                '.' => TokenType::Dot,
                ':' => TokenType::Colon,
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
                        TokenType::Mul
                    }
                }
                '/' => {
                    if self.matchc('/') {
                        let lex: &Lexer = &self.clone();
                        while !self.is_at_end()
                            && *self.peek().ok_or(lexer_expected("unexpected EOF", lex))? != '\n'
                        {
                            self.advance();
                        }
                        return self.scan_token();
                    } else if self.matchc('=') {
                        TokenType::SetDiv
                    } else {
                        TokenType::Div
                    }
                }
                '&' => {
                    if self.matchc('&') {
                        TokenType::And
                    } else {
                        Err(lexer_expected("unexpected character", self))?
                    }
                }
                '|' => {
                    if self.matchc('|') {
                        TokenType::Or
                    } else {
                        TokenType::Xor
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
                    self.start = self.offset;
                    return self.scan_token();
                }
                '\n' => {
                    self.line += 1;
                    self.start = self.offset;
                    return self.scan_token();
                }
                '"' => self.string()?,
                _ => {
                    if Lexer::is_digit(c) {
                        self.number(c)?
                    } else if Lexer::is_alpha(c) {
                        self.identifier(c)?
                    } else {
                        Err(Error::new(vec![Diagnostic::new(
                            format!("unexpected character '{}'", c),
                            self.make_source(),
                        )]))?
                    }
                }
            }
        } else {
            TokenType::Eof
        };
        self.add_token(token_type);
        Ok(())
    }

    fn identifier(&mut self, c: char) -> Result<TokenType> {
        let mut text = String::from(c);
        let lex: &Lexer = &self.clone();
        while !self.is_at_end()
            && Lexer::is_alpha_numeric(*self.peek().ok_or(lexer_expected("unexpected EOF", lex))?)
        {
            if let Some(c) = self.advance() {
                text.push(c)
            }
        }

        Ok(match text.as_str() {
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
            _ => TokenType::Identifier(text),
        })
    }

    fn number(&mut self, c: char) -> Result<TokenType> {
        let mut text = String::from(c);
        let mut is_float = false;
        while !self.is_at_end() {
            let next = if let Some(next) = self.peek() {
                *next
            } else {
                Err(lexer_expected("unexpected EOF", self))?
            };
            if Lexer::is_digit(next) {
                text.push(
                    self.advance()
                        .ok_or(lexer_expected("unexpected EOF, expected a digit", self))?,
                )
            } else if next == '.' {
                if is_float {
                    return Err(lexer_expected("found a second dot in the float", self));
                }
                is_float = true;
                text.push(
                    self.advance()
                        .ok_or(lexer_expected("found EOF, expected a dot ('.')", self))?,
                );
            } else {
                break;
            }
        }

        Ok(TokenType::Number(text.parse().unwrap()))
    }

    fn string(&mut self) -> crate::error::Result<TokenType> {
        let mut text = String::new();
        // FIX:
        while *self
            .peek()
                .unwrap()
            // .ok_or(lexer_expected("got EOF in unterminated String", self))?
            != '"'
        {
            if let Some(c) = self.advance() {
                if c == '\n' {
                    self.line += 1;
                }
                text.push(c);
            }
        }

        // The enclosing ".
        self.advance();

        Ok(TokenType::String(text))
    }

    fn matchc(&mut self, c: char) -> bool {
        match self.peek() {
            Some(p) => {
                if p != &c {
                    return false;
                }
                self.advance();
                true
            }
            _ => false,
        }
    }

    fn peek(&mut self) -> Option<&char> {
        self.source.peek()
    }

    fn is_alpha(c: char) -> bool {
        c.is_alphabetic() || c == '_'
    }

    fn is_digit(c: char) -> bool {
        c.is_ascii_digit()
    }

    fn is_alpha_numeric(c: char) -> bool {
        Lexer::is_alpha(c) || Lexer::is_digit(c)
    }

    fn is_at_end(&mut self) -> bool {
        self.peek().is_none()
    }

    fn advance(&mut self) -> Option<char> {
        self.offset += 1;
        self.source.next()
    }

    /// Public because it is used for error
    pub(crate) fn make_source(&self) -> Source {
        Source::new(self.file.clone(), self.line, self.start, self.offset)
    }

    fn add_token(&mut self, token_type: TokenType) {
        self.tokens
            .push_back(Token::new(token_type, self.make_source()))
    }
}
