use crate::{
    ast::{
        declaration::{Decl, Function, Modifiers, Structure},
        expression::{Expr, ExprKind},
        identifier::Identifier,
        statement::{Stmt, StmtType},
    },
    error::{Result, parser_expected},
    lexer::{Source, Token, TokenType},
    typing::TypeDef,
};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

// Pratt parser inspired after the following block post:
// https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html
pub struct Parser {
    tokens: VecDeque<Token>,
    declarations: Vec<Decl>,
}

impl Parser {
    pub fn new(tokens: VecDeque<Token>) -> Parser {
        Parser {
            tokens,
            declarations: Vec::new(),
        }
    }

    pub fn parse(mut self) -> Result<Vec<Decl>> {
        while self.has_next() {
            let declaration = self.decl();
            self.declarations.push(declaration?);
        }
        Ok(self.declarations)
    }

    pub(crate) fn decl(&mut self) -> Result<Decl> {
        let token = self.advance();
        if token.token_type() == &TokenType::Eof {
            return Err(parser_expected(
                "Found end of File but expected a token",
                token,
            ));
        }
        match token.token_type() {
            TokenType::Const => self.decl_const(),
            TokenType::Extern => self.decl_modifier(Modifiers {
                is_extern: true,
                ..Modifiers::default()
            }),
            TokenType::Io => self.decl_modifier(Modifiers {
                is_io: true,
                ..Modifiers::default()
            }),
            TokenType::Function => self.decl_function(Modifiers::default()),
            TokenType::Import => self.decl_import(),
            TokenType::Struct => self.decl_struct(),
            token_type => Err(parser_expected(
                format!(
                    "Found {:?} expected import, function, struct or const",
                    &token_type
                ),
                token,
            )),
        }
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

    fn decl_modifier(&mut self, modifiers: Modifiers) -> Result<Decl> {
        let token = self.advance();
        match token.token_type() {
            TokenType::Extern => {
                if modifiers.is_extern {
                    Err(parser_expected("function is already extern", token))
                } else {
                    self.decl_modifier(Modifiers {
                        is_extern: true,
                        ..modifiers
                    })
                }
            }
            TokenType::Io => {
                if modifiers.is_io {
                    Err(parser_expected("function is already io", token))
                } else {
                    self.decl_modifier(Modifiers {
                        is_extern: true,
                        ..modifiers
                    })
                }
            }
            TokenType::Function => self.decl_function(modifiers),
            _ => Err(parser_expected(
                "unexpected return type, expected modifier or function",
                token,
            )),
        }
    }

    fn decl_function(&mut self, modifiers: Modifiers) -> Result<Decl> {
        let name = self.advance();
        if !name.token_type().is_identifer() {
            return Err(parser_expected("expected an function identifier", name));
        }

        self.consume(&TokenType::ParenLeft)?;
        let mut params = Vec::new();
        while *self.peek().token_type() != TokenType::ParenRight {
            params.push(self.decl_type_def()?);
            if *self.peek().token_type() != TokenType::ParenRight {
                self.consume(&TokenType::Comma)?;
            }
        }
        let end = self.consume(&TokenType::ParenRight)?;

        let return_type = if *self.peek().token_type() == TokenType::To {
            self.consume(&TokenType::To)?;
            self.decl_type()?
        } else {
            Rc::new(TypeDef::Path(vec![Identifier::new(
                "void".to_string(),
                // TODO: replace source with None, as we can not find any source for an implicit
                // token. Maybe make source optional?
                end.source(),
            )]))
        };

        let body = if modifiers.is_extern {
            Vec::new()
        } else {
            self.decl_body()?
        };

        Ok(Decl::Function(Function {
            name: Identifier::from(name),
            return_type: RefCell::new(return_type),
            params,
            body,
            modifiers,
        }))
    }

    fn decl_body(&mut self) -> Result<Vec<Stmt>> {
        let mut body = Vec::new();
        if self.peek().token_type() == &TokenType::BraceLeft {
            self.consume(&TokenType::BraceLeft)?;
            while *self.peek().token_type() != TokenType::BraceRight {
                body.push(self.stmt()?);
            }
            self.consume(&TokenType::BraceRight)?;
        } else {
            self.consume(&TokenType::Set)?;
            if *self.peek().token_type() == TokenType::Let {
                self.advance();
                while *self.peek().token_type() != TokenType::In {
                    body.push(self.stmt_let(true)?);
                }
                self.consume(&TokenType::In)?;
            }
            let value = self.expr()?;
            body.push(Stmt::new(value.source(), StmtType::Return { value }));
        }
        Ok(body)
    }

    fn decl_import(&mut self) -> Result<Decl> {
        let mut path = Vec::new();
        loop {
            path.push(Identifier::from(self.advance()));
            if *self.peek().token_type() == TokenType::Dot {
                self.advance();
            } else {
                break;
            }
        }
        Ok(Decl::Import(path))
    }

    fn decl_struct(&mut self) -> Result<Decl> {
        let name = self.advance();
        if !name.token_type().is_identifer() {
            return Err(parser_expected("expected a struct identifier", name));
        };
        self.consume(&TokenType::BraceLeft)?;

        let mut fields = Vec::new();
        while self.peek().token_type().is_identifer() {
            fields.push(self.decl_type_def()?);
            if *self.peek().token_type() == TokenType::Comma {
                self.consume(&TokenType::Comma)?;
            }
        }

        self.consume(&TokenType::BraceRight)?;
        Ok(Decl::Struct(Structure {
            name: Identifier::from(name),
            fields,
        }))
    }

    fn decl_type_def(&mut self) -> Result<(Identifier, RefCell<Rc<TypeDef>>)> {
        let name = self.advance();
        if !name.token_type().is_identifer() {
            return Err(parser_expected("expected a type identifier", name));
        }
        self.consume(&TokenType::Colon)?;
        let type_def = self.decl_type()?;
        Ok((Identifier::from(name), RefCell::new(type_def)))
    }

    fn decl_type(&mut self) -> Result<Rc<TypeDef>> {
        let next = self.advance();
        match next.token_type() {
            TokenType::Mul => Ok(Rc::new(TypeDef::RawPointer(self.decl_type()?))),
            TokenType::BracketLeft => {
                let type_def = Rc::new(TypeDef::Array(self.decl_type()?));
                self.consume(&TokenType::BracketRight)?;
                Ok(type_def)
            }
            TokenType::Identifier(_) => {
                let mut path = vec![next.into()];
                while *self.peek().token_type() == TokenType::Dot {
                    // TODO: parse the path
                    self.consume(&TokenType::Dot)?;
                    let id = self.advance().into();
                    path.push(id);
                }
                Ok(Rc::new(TypeDef::Path(path)))
            }
            _ => Err(parser_expected("expected a type definition", next)),
        }
    }

    pub(crate) fn stmt(&mut self) -> Result<Stmt> {
        let peek = self.peek();
        match peek.token_type() {
            TokenType::BraceLeft => self.stmt_block(),
            TokenType::Let => self.stmt_let(false),
            TokenType::Return => {
                let current = self.advance();
                Ok(Stmt::new(
                    current.source(),
                    StmtType::Return {
                        value: self.expr()?,
                    },
                ))
            }
            TokenType::Break => {
                let current = self.advance();
                Ok(Stmt::new(current.source(), StmtType::Break))
            }
            TokenType::For => self.stmt_for(),
            TokenType::While => self.stmt_while(),
            _ => {
                let expr = self.expr()?;
                Ok(Stmt::new(expr.source().clone(), StmtType::ExprStmt(expr)))
            }
        }
    }

    fn stmt_block(&mut self) -> Result<Stmt> {
        let current = self.advance();
        let mut statements = Vec::new();
        while *self.peek().token_type() != TokenType::BraceRight {
            statements.push(Rc::new(self.stmt()?));
        }
        let rhs = self.consume(&TokenType::BraceRight)?;
        Ok(Stmt::new(
            Source::union(&current.source(), &rhs.source()),
            StmtType::Block { statements },
        ))
    }

    /// Parses a single let statement. A let statement is of the form:
    ///
    /// ```text
    /// `let` identifier (`:` type)? `=` expr
    /// ```
    ///
    /// Alternativly, currently only available for functions there exists the
    /// let/in syntax:
    ///
    /// ```text
    /// `let`
    ///   (stmt_let)+
    /// `in`
    ///   expr
    /// ```
    ///
    /// Not every let statement has its own let token. The keyword may already
    /// be consumed by the parser at a previous step. Therefore it may be
    /// skipped by setting the skip let arg to true.
    fn stmt_let(&mut self, skip_let: bool) -> Result<Stmt> {
        let (start, name) = if skip_let {
            let next = self.advance();
            (next.clone(), next)
        } else {
            (self.advance(), self.advance())
        };
        if !name.token_type().is_identifer() {
            return Err(parser_expected("expected a variable identifier", name));
        }
        let var_type = if *self.peek().token_type() != TokenType::Set {
            self.consume(&TokenType::Colon)?;
            let type_name = self.advance();
            if !type_name.token_type().is_identifer() {
                return Err(parser_expected(
                    "expected a type identifier for the variable",
                    type_name,
                ));
            }
            TypeDef::Path(vec![type_name.into()])
        } else {
            TypeDef::Unknown
        };
        self.consume(&TokenType::Set)?;
        let initializer = self.expr()?;
        Ok(Stmt::new(
            Source::union(&start.source(), &initializer.source()),
            StmtType::Let {
                name: Identifier::from(name),
                var_type: RefCell::new(Rc::new(var_type)),
                initializer,
            },
        ))
    }

    fn stmt_for(&mut self) -> Result<Stmt> {
        let current = self.advance();
        self.consume(&TokenType::ParenLeft)?;
        let initializer = Rc::new(self.stmt()?);
        self.consume(&TokenType::Colon)?;
        let condition = self.expr()?;
        self.consume(&TokenType::Colon)?;
        let increment = self.expr()?;
        self.consume(&TokenType::ParenRight)?;
        let body = Rc::new(self.stmt()?);
        Ok(Stmt::new(
            Source::union(&current.source(), &body.source()),
            StmtType::Block {
                statements: vec![
                    initializer,
                    Rc::new(Stmt::new(
                        Source::union(&condition.source(), &body.source()),
                        StmtType::While {
                            condition,
                            body: Rc::new(Stmt::new(
                                body.source(),
                                StmtType::Block {
                                    statements: vec![
                                        body,
                                        Rc::new(Stmt::new(
                                            increment.source(),
                                            StmtType::ExprStmt(increment),
                                        )),
                                    ],
                                },
                            )),
                        },
                    )),
                ],
            },
        ))
    }

    fn stmt_while(&mut self) -> Result<Stmt> {
        let current = self.advance();
        self.consume(&TokenType::ParenLeft)?;
        let condition = self.expr()?;
        self.consume(&TokenType::ParenRight)?;
        let body = Rc::new(self.stmt()?);
        Ok(Stmt::new(
            Source::union(&current.source(), &body.source()),
            StmtType::While { condition, body },
        ))
    }

    pub(crate) fn expr(&mut self) -> Result<Expr> {
        self.expr_bp(0)
    }

    fn expr_bp(&mut self, min_bp: u8) -> Result<Expr> {
        let next = self.advance();
        let mut lhs = self.expr_lhs(next)?;
        while self.has_next() {
            let op = self.peek().token_type();
            if let Some(l_bp) = Parser::postfix_binding_power(op) {
                // If current binding power is below minimum, we return the left hand side
                // expression
                if l_bp < min_bp {
                    break;
                }
                let next = self.advance();
                match next.token_type() {
                    TokenType::BracketLeft => {
                        lhs = self.expr_index(lhs)?;
                    }
                    TokenType::ParenLeft => {
                        lhs = self.expr_call(lhs)?;
                    }
                    unexpected => unreachable!("Unexpected operator: {:?}", unexpected),
                }
            } else if let Some((l_bp, r_bp)) = Parser::infix_binding_power(op) {
                // If current binding power is below minimum, we return the left hand side
                // expression
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

    /**
     * Parses the left hand side of a expression.
     */
    fn expr_lhs(&mut self, next: Token) -> Result<Expr> {
        match next.token_type() {
            TokenType::Bool(_) | TokenType::Number(_) | TokenType::String(_) => {
                self.expr_literal(next)
            }
            TokenType::Identifier(_) => self.expr_identifier(next),
            TokenType::VSelf => self.expr_self(next),
            TokenType::Add | TokenType::Sub | TokenType::Not => self.expr_unary(next),
            TokenType::ParenLeft => self.expr_grouping(),
            TokenType::If => self.expr_if(),
            x => Err(parser_expected(
                format!(
                    "found {:?} but expected if, +, - , number, bool, brace or identifier",
                    x
                ),
                next,
            ))?,
        }
    }

    fn expr_unary(&mut self, op: Token) -> Result<Expr> {
        let r_bp = Parser::prefix_binding_power(op.token_type());
        let rhs = self.expr_bp(r_bp)?;
        Ok(Expr::new(
            Source::union(&op.source(), &rhs.source()),
            ExprKind::Unary {
                operator: op,
                right: Rc::new(rhs),
            },
        ))
    }

    fn expr_binary(&mut self, lhs: Expr, bp: u8) -> Result<Expr> {
        let next = self.advance();
        if *next.token_type() == TokenType::Dot {
            let rhs = self.advance();
            if !rhs.token_type().is_identifer() {
                return Err(parser_expected("expected an identidier on the RHS", rhs));
            }
            Ok(Expr::new(
                Source::union(&lhs.source(), &rhs.source()),
                ExprKind::Get {
                    left: Rc::new(lhs),
                    right: Identifier::from(rhs),
                    lookup: RefCell::new(Rc::new(TypeDef::Unknown)),
                },
            ))
        } else {
            let rhs = self.expr_bp(bp)?;
            Ok(Expr::new(
                Source::union(&lhs.source(), &rhs.source()),
                ExprKind::Binary {
                    left: Rc::new(lhs),
                    operator: next,
                    right: Rc::new(rhs),
                },
            ))
        }
    }

    fn expr_call(&mut self, lhs: Expr) -> Result<Expr> {
        let mut arguments = Vec::new();
        while *self.peek().token_type() != TokenType::ParenRight {
            arguments.push(Rc::new(self.expr()?));
            if *self.peek().token_type() == TokenType::Comma {
                self.consume(&TokenType::Comma)?;
            }
        }
        let rhs = self.consume(&TokenType::ParenRight)?;
        Ok(Expr::new(
            Source::union(&lhs.source(), &rhs.source()),
            ExprKind::Call {
                callee: Rc::new(lhs),
                arguments,
            },
        ))
    }

    fn expr_create_or_index(&mut self, next: Token) -> Result<Expr> {
        self.advance();
        if *self.peek().token_type() == TokenType::BracketRight {
            // No index was given, only a array creation is possible
            self.advance();
            self.expr_create_array(next, None)
        } else {
            // A index was given, can be a sized array or
            let expr = Rc::new(self.expr()?);
            let current = self.consume(&TokenType::BracketRight)?;
            if *self.peek().token_type() == TokenType::BraceLeft {
                self.expr_create_array(next, Some(expr))
            } else {
                let kind = ExprKind::Index {
                    object: Rc::new(Expr::new(
                        current.source(),
                        ExprKind::Variable {
                            name: Identifier::from(next),
                            variable_type: RefCell::new(Rc::new(TypeDef::Unknown)),
                        },
                    )),
                    index: expr,
                    lookup: RefCell::new(Rc::new(TypeDef::Unknown)),
                };
                Ok(Expr::new(current.source(), kind))
            }
        }
    }

    fn expr_create_array(
        &mut self,
        array_type: Token,
        array_size: Option<Rc<Expr>>,
    ) -> Result<Expr> {
        self.consume(&TokenType::BraceLeft)?;
        let mut fields = Vec::new();
        while *self.peek().token_type() != TokenType::BraceRight {
            fields.push(Rc::new(self.expr()?));
            if *self.peek().token_type() != TokenType::BraceRight {
                self.consume(&TokenType::Comma)?;
            }
        }
        let current = self.consume(&TokenType::BraceRight)?;

        Ok(Expr::new(
            Source::union(&array_type.source(), &current.source()),
            ExprKind::CreateArray {
                array_type: RefCell::new(Rc::new(TypeDef::Path(vec![array_type.into()]))),
                array_size,
                fields,
            },
        ))
    }

    fn expr_index(&mut self, lhs: Expr) -> Result<Expr> {
        let rhs = self.expr()?;
        self.consume(&TokenType::BracketRight)?;
        Ok(Expr::new(
            Source::union(&lhs.source(), &rhs.source()),
            ExprKind::Index {
                object: Rc::new(lhs),
                index: Rc::new(rhs),
                lookup: RefCell::new(Rc::new(TypeDef::Unknown)),
            },
        ))
    }

    fn expr_create_struct(&mut self, struct_name: Token) -> Result<Expr> {
        let current = self.consume(&TokenType::BraceLeft)?;
        let mut fields = Vec::new();
        while *self.peek().token_type() != TokenType::BraceRight {
            let name = self.advance();
            if !name.token_type().is_identifer() {
                return Err(parser_expected(
                    "expected field name identifier for the struct",
                    name,
                ));
            }
            self.consume(&TokenType::Set)?;
            let expr = Rc::new(self.expr()?);
            let field = (Identifier::from(name), expr);
            fields.push(field);
            if *self.peek().token_type() != TokenType::BraceRight {
                self.consume(&TokenType::Comma)?;
            }
        }
        self.consume(&TokenType::BraceRight)?;
        Ok(Expr::new(
            Source::union(&struct_name.source(), &current.source()),
            ExprKind::CreateStruct {
                struct_type: RefCell::new(Rc::new(TypeDef::Path(
                    // TODO: include full module path
                    vec![struct_name.into()],
                ))),
                fields,
            },
        ))
    }

    fn expr_grouping(&mut self) -> Result<Expr> {
        // Parse a new expression from the top
        let lhs = self.expr();
        // Consume the ending parenthesis
        self.consume(&TokenType::ParenRight)?;
        lhs
    }

    fn expr_identifier(&mut self, next: Token) -> Result<Expr> {
        match *self.peek().token_type() {
            TokenType::BraceLeft => self.expr_create_struct(next),
            TokenType::BracketLeft => self.expr_create_or_index(next),
            _ => Ok(Expr::new(
                next.source(),
                ExprKind::Variable {
                    name: Identifier::from(next),
                    variable_type: RefCell::new(Rc::new(TypeDef::Unknown)),
                },
            )),
        }
    }

    fn expr_if(&mut self) -> Result<Expr> {
        let current = self.consume(&TokenType::ParenLeft)?;
        let condition = Rc::new(self.expr()?);
        self.consume(&TokenType::ParenRight)?;
        let if_branch = Rc::new(self.stmt()?);
        let (else_branch, source) = if *self.peek().token_type() == TokenType::Else {
            self.consume(&TokenType::Else)?;
            let else_branch = self.stmt()?;
            let source = Source::union(&current.source(), &else_branch.source());
            (Some(Rc::new(else_branch)), source)
        } else {
            let source = Source::union(&current.source(), &if_branch.source());
            (None, source)
        };
        Ok(Expr::new(
            source,
            ExprKind::If {
                condition,
                if_branch,
                else_branch,
                expression_type: RefCell::new(Rc::new(TypeDef::Unknown)),
            },
        ))
    }

    fn expr_literal(&mut self, next: Token) -> Result<Expr> {
        Ok(Expr::new(next.source(), ExprKind::Literal(next)))
    }

    fn expr_self(&mut self, next: Token) -> Result<Expr> {
        Ok(Expr::new(
            next.source(),
            ExprKind::Variable {
                name: Identifier::from(next),
                variable_type: RefCell::new(Rc::new(TypeDef::Unknown)),
            },
        ))
    }

    fn prefix_binding_power(operator: &TokenType) -> u8 {
        match operator {
            TokenType::Not => 1,
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
            // The BraceRight token type is not included here,
            // because we expect that it should only end a statement.
            TokenType::BracketLeft | TokenType::ParenLeft => Some(11),
            _ => None,
        }
    }

    fn consume(&mut self, expected: &TokenType) -> Result<Token> {
        let next = self.advance();
        if next.token_type() != expected {
            // The consumed token was not of the expected type
            return Err(crate::error::parser_expected(
                format!(
                    "expected {:?} but found {:?}",
                    expected.clone(),
                    next.token_type().clone()
                ),
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
