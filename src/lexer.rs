use std::collections::VecDeque;
use std::convert::Into;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone)]
pub struct Token {
    token_type: TokenType,
    line: u32,
    column: u32,
}

impl Token {
    pub fn new(token_type: TokenType, line: u32, column: u32) -> Token {
        Token {
            token_type,
            line,
            column,
        }
    }

    pub fn get_type(&self) -> &TokenType {
        &self.token_type
    }
}

impl Into<TokenType> for Token {
    fn into(self) -> TokenType {
        self.token_type
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Brackets
    ParenLeft,    // (
    ParenRight,   // )
    BraceLeft,    // {
    BraceRight,   // }
    BracketLeft,  // [
    BracketRight, // ]
    Dot,
    Comma,
    Colon,

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
    Return,
    Break,

    // Literal
    Bool(bool),
    Number(i32),
    String(String),
    Identifier(String),

    // Misc
    Eof,
}

impl TokenType {
    pub fn is_identifer(&self) -> bool {
        match self {
            Self::Identifier(_) => true,
            _ => false,
        }
    }
}

pub struct Lexer<'a> {
    source: Peekable<Chars<'a>>,
    tokens: VecDeque<Token>,
    line: u32,
    column: u32,
}

impl Lexer<'_> {
    pub fn new<'a>(source: Chars<'a>) -> Lexer<'a> {
        Lexer {
            source: source.peekable(),
            tokens: VecDeque::new(),
            line: 1,
            column: 0,
        }
    }

    pub fn scan(mut self) -> VecDeque<Token> {
        while !self.is_at_end() {
            self.scan_token();
        }
        self.add_token(TokenType::Eof);
        self.tokens
    }

    fn scan_token(&mut self) {
        match self.advance() {
            Some(c) => match c {
                '(' => self.add_token(TokenType::ParenLeft),
                ')' => self.add_token(TokenType::ParenRight),
                '{' => self.add_token(TokenType::BraceLeft),
                '}' => self.add_token(TokenType::BraceRight),
                '[' => self.add_token(TokenType::BracketLeft),
                ']' => self.add_token(TokenType::BracketRight),
                ',' => self.add_token(TokenType::Comma),
                '.' => self.add_token(TokenType::Dot),
                ':' => self.add_token(TokenType::Colon),
                '+' => {
                    let token_type = if self.matchc('=') {
                        TokenType::SetAdd
                    } else {
                        TokenType::Add
                    };
                    self.add_token(token_type);
                }
                '-' => {
                    let token_type = if self.matchc('=') {
                        TokenType::SetSub
                    } else {
                        TokenType::Sub
                    };
                    self.add_token(token_type);
                }
                '*' => {
                    let token_type = if self.matchc('=') {
                        TokenType::SetMul
                    } else {
                        TokenType::Mul
                    };
                    self.add_token(token_type);
                }
                '/' => {
                    if self.matchc('/') {
                        while *self.peek().unwrap() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    } else {
                        let token_type = if self.matchc('=') {
                            TokenType::SetDiv
                        } else {
                            TokenType::Div
                        };
                        self.add_token(token_type);
                    }
                }
                '&' => {
                    if self.matchc('&') {
                        self.add_token(TokenType::And);
                    } else {
                        panic!("Unexpected character.");
                    }
                }
                '|' => {
                    let token_type = if self.matchc('|') {
                        TokenType::Or
                    } else {
                        TokenType::Xor
                    };
                    self.add_token(token_type);
                }
                '=' => {
                    let token_type = if self.matchc('=') {
                        TokenType::Eq
                    } else {
                        TokenType::Set
                    };
                    self.add_token(token_type);
                }
                '!' => {
                    let token_type = if self.matchc('=') {
                        TokenType::Neq
                    } else {
                        TokenType::Not
                    };
                    self.add_token(token_type);
                }
                '>' => {
                    let token_type = if self.matchc('=') {
                        TokenType::Geq
                    } else {
                        TokenType::Gre
                    };
                    self.add_token(token_type);
                }
                '<' => {
                    let token_type = if self.matchc('=') {
                        TokenType::Leq
                    } else {
                        TokenType::Low
                    };
                    self.add_token(token_type);
                }
                ' ' | '\r' | '\t' => {}
                '\n' => {
                    self.line += 1;
                    self.column = 0;
                }
                '"' => self.string(),
                _ => {
                    if Lexer::is_digit(c) {
                        self.number(c);
                    } else if Lexer::is_alpha(c) {
                        self.identifier(c);
                    } else {
                        panic!("Unexpected character '{}'.", c)
                    }
                }
            },
            _ => {}
        }
    }

    fn identifier(&mut self, c: char) {
        let mut text = String::from(c);
        while Lexer::is_alpha_numeric(*self.peek().unwrap()) {
            match self.advance() {
                Some(c) => {
                    text.push(c);
                }
                _ => {}
            };
        }

        let token_type = match text.as_str() {
            "import" => TokenType::Import,
            "struct" => TokenType::Struct,
            "fn" => TokenType::Function,
            "const" => TokenType::Const,
            "let" => TokenType::Let,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "for" => TokenType::For,
            "while" => TokenType::While,
            "return" => TokenType::Return,
            "break" => TokenType::Break,
            _ => TokenType::Identifier(text),
        };

        self.add_token(token_type);
    }

    fn number(&mut self, c: char) {
        let mut text = String::from(c);
        while !self.is_at_end() && Lexer::is_digit(*self.peek().unwrap()) {
            match self.advance() {
                Some(c) => {
                    text.push(c);
                }
                _ => {}
            }
        }

        self.add_token(TokenType::Number(text.parse().unwrap()));
    }

    fn string(&mut self) {
        let mut text = String::new();
        while !self.is_at_end() && *self.peek().unwrap() != '"' {
            match self.advance() {
                Some(c) => {
                    if c == '\n' {
                        self.line += 1;
                    }
                    text.push(c);
                }
                _ => {}
            }
        }

        if self.is_at_end() {
            panic!("Unterminated string.");
        }

        // The enclosing ".
        self.advance();

        self.add_token(TokenType::String(text));
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
        self.peek() == None
    }

    fn advance(&mut self) -> Option<char> {
        self.column += 1;
        self.source.next()
    }

    fn add_token(&mut self, token_type: TokenType) {
        self.tokens
            .push_back(Token::new(token_type, self.line, self.column))
    }
}
