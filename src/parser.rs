use crate::{
    ast::{Decl, Expr, Identifier, Stmt},
    error::{ParserError, Result, parser_expected},
    lexer::{Token, TokenType},
    typing::TypeDef,
};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

// Pratt parser inspired after the following block post:
// https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html
#[derive(Debug)]
pub struct Parser<'a> {
    tokens: VecDeque<Token>,
    declarations: Vec<Decl>,
    source: &'a str,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: VecDeque<Token>, source: &str) -> Parser<'_> {
        Parser {
            tokens,
            declarations: Vec::new(),
            source,
        }
    }

    // pub fn get_source(&self) -> &'a str {
    //     self.source
    // }

    pub fn parse(mut self) -> Result<Vec<Decl>> {
        while self.has_next() {
            let declaration = self.decl();
            self.declarations.push(declaration?);
        }
        Ok(self.declarations)
    }

    pub(crate) fn decl(&mut self) -> Result<Decl> {
        let token = self.advance();
        if token.get_type() == &TokenType::Eof {
            return Err(parser_expected(
                ParserError::UnexpectedToken(TokenType::Eof, "any other token".to_string()),
                self,
                token,
            ));
        }
        match token.get_type() {
            TokenType::Import => self.decl_import(),
            TokenType::Extern => self.decl_extern(),
            TokenType::Function => self.decl_function(false),
            TokenType::Struct => self.decl_struct(),
            TokenType::Const => self.decl_const(),
            t => Err(parser_expected(
                ParserError::UnexpectedToken(
                    t.clone(),
                    "import, function, struct, const".to_string(),
                ),
                self,
                token,
            )),
        }
    }

    fn decl_import(&mut self) -> Result<Decl> {
        let mut path = Vec::new();
        loop {
            path.push(Identifier::from(self.advance()));
            if *self.peek().get_type() == TokenType::Dot {
                self.advance();
            } else {
                break;
            }
        }
        Ok(Decl::Import(path))
    }

    fn decl_extern(&mut self) -> Result<Decl> {
        let token = self.advance();
        match token.get_type() {
            TokenType::Function => self.decl_function(true),
            _ => todo!("unimplemented decl: {:?}", token),
        }
    }

    fn decl_function(&mut self, is_extern: bool) -> Result<Decl> {
        let name = self.advance();
        if !name.get_type().is_identifer() {
            return Err(parser_expected(ParserError::Name, self, name));
        }

        self.consume(&TokenType::ParenLeft)?;
        let mut params = Vec::new();
        while *self.peek().get_type() != TokenType::ParenRight {
            params.push(self.decl_type_def()?);
            if *self.peek().get_type() != TokenType::ParenRight {
                self.consume(&TokenType::Comma)?;
            }
        }
        self.consume(&TokenType::ParenRight)?;

        let return_type = if *self.peek().get_type() == TokenType::Colon {
            self.consume(&TokenType::Colon)?;
            let type_name = self.advance();
            if !type_name.get_type().is_identifer() {
                return Err(parser_expected(ParserError::Name, self, type_name));
            }
            TypeDef::Lazy(type_name.identifier().to_string())
        } else {
            TypeDef::Lazy("void".to_string())
        };

        let mut body = Vec::new();
        if !is_extern {
            self.consume(&TokenType::BraceLeft)?;
            while *self.peek().get_type() != TokenType::BraceRight {
                body.push(self.stmt()?);
            }
            self.consume(&TokenType::BraceRight)?;
        }

        Ok(Decl::Function {
            name: Identifier::from(name),
            return_type: RefCell::new(Rc::new(return_type)),
            params,
            body,
            is_extern,
        })
    }

    fn decl_struct(&mut self) -> Result<Decl> {
        let name = self.advance();
        if !name.get_type().is_identifer() {
            return Err(parser_expected(ParserError::Name, self, name));
        };
        self.consume(&TokenType::BraceLeft)?;

        let mut fields = Vec::new();
        while self.peek().get_type().is_identifer() {
            fields.push(self.decl_type_def()?);
            if *self.peek().get_type() == TokenType::Comma {
                self.consume(&TokenType::Comma)?;
            }
        }

        let mut methods = Vec::new();
        while *self.peek().get_type() != TokenType::BraceRight {
            self.consume(&TokenType::Function)?;
            methods.push(Rc::new(self.decl_function(false)?));
        }

        self.consume(&TokenType::BraceRight)?;
        Ok(Decl::Struct {
            name: Identifier::from(name),
            fields,
            methods,
        })
    }

    fn decl_const(&mut self) -> Result<Decl> {
        let (name, var_type) = self.decl_type_def()?;
        self.consume(&TokenType::Set)?;
        let initializer = self.expr()?;
        Ok(Decl::Const {
            name,
            var_type,
            initializer,
        })
    }

    fn decl_type_def(&mut self) -> Result<(Identifier, RefCell<Rc<TypeDef>>)> {
        let name = self.advance();
        if !name.get_type().is_identifer() {
            return Err(parser_expected(ParserError::Type, self, name));
        }
        self.consume(&TokenType::Colon)?;
        let type_name = self.advance();
        if !type_name.get_type().is_identifer() {
            return Err(parser_expected(ParserError::Type, self, type_name));
        }
        Ok((
            Identifier::from(name),
            RefCell::new(Rc::new(TypeDef::Lazy(type_name.identifier().to_string()))),
        ))
    }

    pub(crate) fn stmt(&mut self) -> Result<Stmt> {
        match self.peek().get_type() {
            TokenType::BraceLeft => self.stmt_block(),
            TokenType::Let => self.stmt_let(),
            TokenType::Return => {
                self.advance();
                Ok(Stmt::Return {
                    value: self.expr()?,
                })
            }
            TokenType::Break => {
                self.advance();
                Ok(Stmt::Break)
            }
            TokenType::While => self.stmt_while(),
            _ => Ok(Stmt::ExprStmt(self.expr()?)),
        }
    }

    fn stmt_block(&mut self) -> Result<Stmt> {
        self.advance();
        let mut statements = Vec::new();
        while *self.peek().get_type() != TokenType::BraceRight {
            statements.push(Rc::new(self.stmt()?));
        }
        self.consume(&TokenType::BraceRight)?;
        Ok(Stmt::Block { statements })
    }

    fn stmt_let(&mut self) -> Result<Stmt> {
        self.advance();
        let name = self.advance();
        if !name.get_type().is_identifer() {
            return Err(parser_expected(ParserError::Name, self, name));
        }
        let var_type = if *self.peek().get_type() != TokenType::Set {
            self.consume(&TokenType::Colon)?;
            let type_name = self.advance();
            if !type_name.get_type().is_identifer() {
                return Err(parser_expected(ParserError::Name, self, type_name));
            }
            TypeDef::Lazy(type_name.identifier().to_string())
        } else {
            TypeDef::Unknown
        };
        self.consume(&TokenType::Set)?;
        let initializer = self.expr()?;
        Ok(Stmt::Let {
            name: Identifier::from(name),
            var_type: RefCell::new(Rc::new(var_type)),
            initializer,
        })
    }

    fn stmt_while(&mut self) -> Result<Stmt> {
        self.advance();
        self.consume(&TokenType::ParenLeft)?;
        let condition = self.expr()?;
        self.consume(&TokenType::ParenRight)?;
        let body = Rc::new(self.stmt()?);
        Ok(Stmt::While { condition, body })
    }

    pub(crate) fn expr(&mut self) -> Result<Expr> {
        self.expr_bp(0)
    }

    fn expr_bp(&mut self, min_bp: u8) -> Result<Expr> {
        let next = self.advance();
        let mut lhs = match next.get_type() {
            TokenType::Bool(_) | TokenType::Number(_) | TokenType::String(_) => Expr::Literal(next),
            TokenType::Identifier(_) => {
                if *self.peek().get_type() == TokenType::BraceLeft {
                    self.expr_create_struct(next)?
                } else if *self.peek().get_type() == TokenType::BracketLeft {
                    self.advance();
                    // The code below is needed to check if we create a new array or want to index a field.
                    if *self.peek().get_type() == TokenType::BracketRight {
                        self.advance();
                        dbg!("create array");
                        self.expr_create_array(next, None)?
                    } else {
                        let expr = Rc::new(self.expr()?);
                        self.consume(&TokenType::BracketRight)?;
                        if *self.peek().get_type() == TokenType::BraceLeft {
                            self.expr_create_array(next, Some(expr))?
                        } else {
                            Expr::Index {
                                object: Rc::new(Expr::Variable {
                                    name: Identifier::from(next),
                                    variable_type: RefCell::new(Rc::new(TypeDef::Unknown)),
                                }),
                                index: expr,
                                lookup: RefCell::new(Rc::new(TypeDef::Unknown)),
                            }
                        }
                    }
                } else {
                    Expr::Variable {
                        name: Identifier::from(next),
                        variable_type: RefCell::new(Rc::new(TypeDef::Unknown)),
                    }
                }
            }
            TokenType::VSelf => Expr::Variable {
                name: Identifier::from(next),
                variable_type: RefCell::new(Rc::new(TypeDef::Unknown)),
            },
            TokenType::Add | TokenType::Sub | TokenType::Not => self.expr_unary(next)?,
            TokenType::ParenLeft => self.expr_grouping()?,
            TokenType::If => self.expr_if()?,
            x => Err(parser_expected(
                ParserError::UnexpectedToken(
                    x.clone(),
                    "if, +, -, number, bool, identifier".to_string(),
                ),
                self,
                next,
            ))?,
            // _ => todo!("unexpected token in expression: {:?}", next),
        };

        while self.has_next() {
            let op = self.peek().get_type();
            if let Some(l_bp) = Parser::postfix_binding_power(op) {
                // If current binding power is below minimum, we return the left hand side expression
                if l_bp < min_bp {
                    break;
                }
                let next = self.advance();
                match next.get_type() {
                    TokenType::BracketLeft => {
                        lhs = self.expr_index(lhs)?;
                    }
                    TokenType::ParenLeft => {
                        lhs = self.expr_call(lhs)?;
                    }
                    unexpected => unreachable!("Unexpected operator: {:?}", unexpected),
                }
            } else if let Some((l_bp, r_bp)) = Parser::infix_binding_power(op) {
                // If current binding power is below minimum, we return the left hand side expression
                if l_bp < min_bp {
                    break;
                }
                lhs = self.expr_binary(lhs, r_bp)?;
            } else {
                // We reached a token that is not an operator and stop
                break;
            }
        }

        Ok(lhs)
    }

    fn expr_grouping(&mut self) -> Result<Expr> {
        // Parse a new expression from the top
        let lhs = self.expr();
        // Consume the ending parenthesis
        self.consume(&TokenType::ParenRight)?;
        lhs
    }

    fn expr_index(&mut self, lhs: Expr) -> Result<Expr> {
        let rhs = self.expr()?;
        self.consume(&TokenType::BracketRight)?;
        Ok(Expr::Index {
            object: Rc::new(lhs),
            index: Rc::new(rhs),
            lookup: RefCell::new(Rc::new(TypeDef::Unknown)),
        })
    }

    fn expr_call(&mut self, lhs: Expr) -> Result<Expr> {
        let mut arguments = Vec::new();
        while *self.peek().get_type() != TokenType::ParenRight {
            arguments.push(Rc::new(self.expr()?));
            if *self.peek().get_type() == TokenType::Comma {
                self.consume(&TokenType::Comma)?;
            }
        }
        self.consume(&TokenType::ParenRight)?;
        Ok(Expr::Call {
            callee: Rc::new(lhs),
            arguments,
        })
    }

    fn expr_unary(&mut self, op: Token) -> Result<Expr> {
        let r_bp = Parser::prefix_binding_power(op.get_type());
        let rhs = self.expr_bp(r_bp)?;
        Ok(Expr::Unary {
            operator: op,
            right: Rc::new(rhs),
        })
    }

    fn expr_binary(&mut self, lhs: Expr, bp: u8) -> Result<Expr> {
        let next = self.advance();
        if *next.get_type() == TokenType::Dot {
            let rhs = self.advance();
            if !rhs.get_type().is_identifer() {
                return Err(parser_expected(ParserError::Name, self, rhs));
            }
            Ok(Expr::Get {
                left: Rc::new(lhs),
                right: Identifier::from(rhs),
                lookup: RefCell::new(Rc::new(TypeDef::Unknown)),
            })
        } else {
            let rhs = self.expr_bp(bp)?;
            Ok(Expr::Binary {
                left: Rc::new(lhs),
                operator: next,
                right: Rc::new(rhs),
            })
        }
    }

    fn expr_create_array(
        &mut self,
        array_type: Token,
        array_size: Option<Rc<Expr>>,
    ) -> Result<Expr> {
        self.consume(&TokenType::BraceLeft)?;
        let mut fields = Vec::new();
        while *self.peek().get_type() != TokenType::BraceRight {
            fields.push(Rc::new(self.expr()?));
            if *self.peek().get_type() != TokenType::BraceRight {
                self.consume(&TokenType::Comma)?;
            }
        }
        self.consume(&TokenType::BraceRight)?;

        Ok(Expr::CreateArray {
            array_type: RefCell::new(Rc::new(TypeDef::Lazy(array_type.identifier().to_string()))),
            array_size,
            fields,
        })
    }

    fn expr_create_struct(&mut self, struct_name: Token) -> Result<Expr> {
        self.consume(&TokenType::BraceLeft)?;
        let mut fields = Vec::new();
        while *self.peek().get_type() != TokenType::BraceRight {
            let name = self.advance();
            if !name.get_type().is_identifer() {
                return Err(parser_expected(ParserError::Name, self, name));
            }
            self.consume(&TokenType::Set)?;
            let expr = Rc::new(self.expr()?);
            let field = (Identifier::from(name), expr);
            fields.push(field);
            if *self.peek().get_type() != TokenType::BraceRight {
                self.consume(&TokenType::Comma)?;
            }
        }
        self.consume(&TokenType::BraceRight)?;
        Ok(Expr::CreateStruct {
            struct_type: RefCell::new(Rc::new(TypeDef::Lazy(struct_name.identifier().to_string()))),
            fields,
        })
    }

    fn expr_if(&mut self) -> Result<Expr> {
        self.consume(&TokenType::ParenLeft)?;
        let condition = Rc::new(self.expr()?);
        self.consume(&TokenType::ParenRight)?;
        let if_branch = Rc::new(self.stmt()?);
        let else_branch = if *self.peek().get_type() == TokenType::Else {
            self.consume(&TokenType::Else)?;
            Some(Rc::new(self.stmt()?))
        } else {
            None
        };
        Ok(Expr::If {
            condition,
            if_branch,
            else_branch,
            expression_type: RefCell::new(Rc::new(TypeDef::Unknown)),
        })
    }

    fn prefix_binding_power(operator: &TokenType) -> u8 {
        match operator {
            TokenType::Add | TokenType::Sub => 6,
            unexpected => unreachable!("Unexpected operator: {:?}", unexpected),
        }
    }

    fn infix_binding_power(operator: &TokenType) -> Option<(u8, u8)> {
        match operator {
            TokenType::Set
            | TokenType::SetAdd
            | TokenType::SetSub
            | TokenType::SetMul
            | TokenType::SetDiv => Some((1, 2)),
            TokenType::And | TokenType::Or | TokenType::Xor => Some((3, 4)),
            TokenType::Low
            | TokenType::Leq
            | TokenType::Eq
            | TokenType::Gre
            | TokenType::Geq
            | TokenType::Neq => Some((5, 6)),
            TokenType::Add | TokenType::Sub => Some((7, 8)),
            TokenType::Mul | TokenType::Div => Some((9, 10)),
            TokenType::Dot => Some((12, 11)),
            _ => None,
        }
    }

    fn postfix_binding_power(operator: &TokenType) -> Option<u8> {
        match operator {
            TokenType::BracketLeft | TokenType::ParenLeft | TokenType::Not => Some(11),
            _ => None,
        }
    }

    fn consume(&mut self, expected: &TokenType) -> Result<Token> {
        let next = self.advance();
        if next.get_type() != expected {
            return Err(crate::error::parser_expected(
                ParserError::SpecificTokenType(expected.clone(), next.get_type().clone()),
                self,
                next,
            ));
        }
        Ok(next)
    }

    fn advance(&mut self) -> Token {
        self.tokens.pop_front().expect("Expected next token")
    }

    fn peek(&self) -> &Token {
        self.tokens.front().expect("Expected next token")
    }

    fn has_next(&self) -> bool {
        self.tokens.len() > 1
    }
}
