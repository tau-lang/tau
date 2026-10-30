use crate::{
    ast::{
        Location, UntypedDecl, UntypedExpr, UntypedStmt,
        declaration::{DeclFunc, DeclKind, DeclStruct, FuncModifiers},
        identifier::Identifier,
        origin::ExprOrigin,
        prelude::*,
        statement::StmtKind,
    },
    error::{Error, Result, diagnostic, error},
    lexer::{Token, TokenType},
    type_::{Type, prelude::*},
};
use std::{boxed::Box, iter::Iterator, sync::Arc};
use symbol_table::{GlobalSymbol, static_symbol};

// Pratt parser inspired after the following block post:
// https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html
pub struct Parser<I: Iterator<Item = Result<Token>>> {
    /// The token iterator. This is generelly the Lexer, but by making it
    /// generic, we allow any type of token iterator.
    tokens: I,
    /// All declaration's that we parsed are added to this collection.
    declarations: Vec<UntypedDecl>,
    /// The last token that the parser read. This is used for the peek() method.
    /// Why do we implement peek() ourself and don't use Peekable instead? The
    /// issue with Peekable is, that it does not propagate the errors, which is
    /// essential for the parser.
    last: Option<Token>,
}

impl<I: Iterator<Item = Result<Token>>> Parser<I> {
    pub fn new(tokens: I) -> Parser<I> {
        Parser {
            tokens: tokens,
            declarations: Vec::new(),
            last: None,
        }
    }

    pub fn parse(mut self) -> Result<Vec<UntypedDecl>> {
        while self.has_next() {
            let declaration = self.decl()?;
            self.declarations.push(declaration);
        }
        Ok(self.declarations)
    }

    pub(crate) fn decl(&mut self) -> Result<UntypedDecl> {
        let token = self.advance()?;
        if token.token_type() == &TokenType::Eof {
            return error(diagnostic("Found end of File but expected a token", &token));
        }
        match token.token_type() {
            TokenType::Const => self.decl_const(token),
            TokenType::Extern => self.decl_modifier(
                token,
                FuncModifiers {
                    is_extern: true,
                    ..Default::default()
                },
            ),
            TokenType::Io => self.decl_modifier(
                token,
                FuncModifiers {
                    is_io: true,
                    ..Default::default()
                },
            ),
            TokenType::Function => self.decl_function(token, FuncModifiers::default()),
            TokenType::Import => self.decl_import(token),
            TokenType::Struct => self.decl_struct(token),
            token_type => error(diagnostic(
                format!("Found {:?} expected declaration", &token_type),
                &token,
            )),
        }
    }

    fn decl_const(&mut self, start: Token) -> Result<UntypedDecl> {
        let (name, type_) = self.decl_type_def()?;
        self.consume(&TokenType::Set)?;
        let initializer = self.expr()?;
        let end = self.consume(&TokenType::Semicolon)?;
        let source = start.merge(&end);
        Ok(UntypedDecl::new(
            DeclKind::Const {
                name,
                type_,
                initializer,
            },
            source,
        ))
    }

    fn decl_modifier(&mut self, start: Token, modifiers: FuncModifiers) -> Result<UntypedDecl> {
        let token = self.advance()?;
        match token.token_type() {
            TokenType::Extern => {
                if modifiers.is_extern {
                    error(diagnostic("function is already extern", &token))
                } else {
                    self.decl_modifier(
                        start,
                        FuncModifiers {
                            is_extern: true,
                            ..modifiers
                        },
                    )
                }
            }
            TokenType::Io => {
                if modifiers.is_io {
                    error(diagnostic("function is already io", &token))
                } else {
                    self.decl_modifier(
                        start,
                        FuncModifiers {
                            is_extern: true,
                            ..modifiers
                        },
                    )
                }
            }
            TokenType::Function => self.decl_function(start, modifiers),
            _ => error(diagnostic(
                "unexpected return type, expected modifier or function",
                &token,
            )),
        }
    }

    fn decl_function(&mut self, start: Token, modifiers: FuncModifiers) -> Result<UntypedDecl> {
        let name = self.advance()?;
        if !name.token_type().is_identifier() {
            return error(diagnostic("expected an function identifier", &name));
        }

        self.consume(&TokenType::ParenLeft)?;
        let mut params = Vec::new();
        while *self.peek()?.token_type() != TokenType::ParenRight {
            params.push(self.decl_type_def()?);
            if *self.peek()?.token_type() != TokenType::ParenRight {
                self.consume(&TokenType::Comma)?;
            }
        }
        let end = self.consume(&TokenType::ParenRight)?;

        let return_type = if *self.peek()?.token_type() == TokenType::To {
            self.consume(&TokenType::To)?;
            self.decl_type()?
        } else {
            Arc::new(Type::Named {
                name: static_symbol!("void"),
                arguments: vec![],
            })
        };

        let body = if modifiers.is_extern {
            Default::default()
        } else {
            self.decl_body()?
        };

        let source = start.merge(&end);
        Ok(UntypedDecl::new(
            DeclKind::Func(DeclFunc {
                name: Identifier::from(name),
                return_type,
                params,
                body,
                modifiers,
            }),
            source,
        ))
    }

    fn decl_body(&mut self) -> Result<Vec<UntypedStmt>> {
        let mut body = Vec::new();
        if self.peek()?.token_type() == &TokenType::BraceLeft {
            self.consume(&TokenType::BraceLeft)?;
            while *self.peek()?.token_type() != TokenType::BraceRight {
                body.push(self.stmt()?);
            }
            self.consume(&TokenType::BraceRight)?;
        } else {
            self.consume(&TokenType::Set)?;
            if *self.peek()?.token_type() == TokenType::Let {
                self.advance()?;
                while *self.peek()?.token_type() != TokenType::In {
                    body.push(self.stmt_let(true)?);
                }
                self.consume(&TokenType::In)?;
            }
            let value = self.expr()?;
            let source = value.source().clone();
            body.push(UntypedStmt::new(StmtKind::Return { value }, source));
        }
        Ok(body)
    }

    fn decl_import(&mut self, start: Token) -> Result<UntypedDecl> {
        let mut path = Vec::new();
        loop {
            path.push(Identifier::from(self.advance()?));
            if *self.peek()?.token_type() == TokenType::Dot {
                self.advance()?;
            } else {
                break;
            }
        }
        let end = self.consume(&TokenType::Semicolon)?;
        let source = start.merge(&end);
        Ok(UntypedDecl::new(DeclKind::Import(path), source))
    }

    fn decl_struct(&mut self, start: Token) -> Result<UntypedDecl> {
        let name = self.advance()?;
        if !name.token_type().is_identifier() {
            return error(diagnostic("expected a struct identifier", &name));
        };
        self.consume(&TokenType::BraceLeft)?;

        let mut fields = Vec::new();
        while self.peek()?.token_type().is_identifier() {
            let (field_name, field_type) = self.decl_type_def()?;
            fields.push((field_name, field_type));
            if *self.peek()?.token_type() == TokenType::Comma {
                self.consume(&TokenType::Comma)?;
            }
        }

        let end = self.consume(&TokenType::BraceRight)?;
        let source = start.merge(&end);
        Ok(UntypedDecl::new(
            DeclKind::Struct(DeclStruct {
                name: Identifier::from(name),
                fields,
            }),
            source,
        ))
    }

    fn decl_type_def(&mut self) -> Result<(Identifier, Arc<Type>)> {
        let name = self.advance()?;
        if !name.token_type().is_identifier() {
            return error(diagnostic("expected a type identifier", &name));
        }
        self.consume(&TokenType::Colon)?;
        let type_def = self.decl_type()?;
        Ok((Identifier::from(name), type_def))
    }

    fn decl_type(&mut self) -> Result<Arc<Type>> {
        let next = self.advance()?;
        match next.token_type() {
            TokenType::Star => Ok(Arc::new(Type::Ptr {
                unchecked: true,
                type_: self.decl_type()?,
            })),
            TokenType::BracketLeft => {
                let type_def = Arc::new(Type::Ptr {
                    unchecked: false,
                    type_: self.decl_type()?,
                });
                self.consume(&TokenType::BracketRight)?;
                Ok(type_def)
            }
            TokenType::Identifier(_) => {
                // We may add arguments when we support generics. Until then the
                // arguments are always empty.
                Ok(Arc::new(Type::Named {
                    name: GlobalSymbol::from(Identifier::from(next)),
                    arguments: vec![],
                }))
            }
            TokenType::ParenLeft => {
                let mut arguments = vec![self.decl_type()?];
                while *self.peek()?.token_type() == TokenType::Comma {
                    self.consume(&TokenType::Comma)?;
                    let argument = self.decl_type()?;
                    arguments.push(argument);
                }
                self.consume(&TokenType::To)?;
                let return_ = self.decl_type()?;
                self.consume(&TokenType::ParenRight)?;
                Ok(Arc::new(Type::Fn { arguments, return_ }))
            }
            _ => error(diagnostic("expected a type definition", &next)),
        }
    }

    pub(crate) fn stmt(&mut self) -> Result<UntypedStmt> {
        let peek = self.peek()?;
        match peek.token_type() {
            TokenType::BraceLeft => self.stmt_block(),
            TokenType::Let => self.stmt_let(false),
            TokenType::Return => {
                let start = self.advance()?;
                let expr = self.expr()?;
                let end = self.consume(&TokenType::Semicolon)?;
                let source = start.merge(&end);
                Ok(UntypedStmt::new(StmtKind::Return { value: expr }, source))
            }
            TokenType::Break => {
                let current = self.advance()?;
                let source = current.source().clone();
                Ok(UntypedStmt::new(StmtKind::Break, source))
            }
            TokenType::For => self.stmt_for(),
            TokenType::While => self.stmt_while(),
            _ => {
                let expr = self.expr()?;
                if *self.peek()?.token_type() == TokenType::Semicolon {
                    self.advance()?;
                }
                let source = expr.source().clone();
                Ok(UntypedStmt::new(StmtKind::ExprStmt(expr), source))
            }
        }
    }

    fn stmt_block(&mut self) -> Result<UntypedStmt> {
        let start = self.advance()?;
        let mut statements = Vec::new();
        while *self.peek()?.token_type() != TokenType::BraceRight {
            statements.push(self.stmt()?);
        }
        let end = self.consume(&TokenType::BraceRight)?;
        let source = start.merge(&end);
        Ok(UntypedStmt::new(StmtKind::Block { statements }, source))
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
    fn stmt_let(&mut self, skip_let: bool) -> Result<UntypedStmt> {
        let (start, name) = if skip_let {
            let next = self.advance()?;
            (next.clone(), next)
        } else {
            (self.advance()?, self.advance()?)
        };
        if !name.token_type().is_identifier() {
            return error(diagnostic("expected a variable identifier", &name));
        }
        let declared = if *self.peek()?.token_type() != TokenType::Set {
            self.consume(&TokenType::Colon)?;
            Some(self.decl_type()?)
        } else {
            None
        };
        self.consume(&TokenType::Set)?;
        let initializer = self.expr()?;
        self.consume(&TokenType::Semicolon)?;
        let source = start.merge(&initializer);
        Ok(UntypedStmt::new(
            StmtKind::Let {
                name: Identifier::from(name),
                declared,
                initializer,
            },
            source,
        ))
    }

    fn stmt_for(&mut self) -> Result<UntypedStmt> {
        let start = self.advance()?;
        let initializer = self.stmt()?;
        self.consume(&TokenType::Semicolon)?;
        let condition = self.expr()?;
        self.consume(&TokenType::Semicolon)?;
        let increment = self.expr()?;
        let body = self.stmt_block()?;
        let source = increment.source();
        let increment = expr_stmt(increment, source);
        let source = body.source();
        let body = block(vec![body, increment], source);
        let source = start.merge(&body);
        let statements = vec![initializer, while_(condition, body, source)];
        Ok(block(statements, source))
    }

    fn stmt_while(&mut self) -> Result<UntypedStmt> {
        let start = self.advance()?;
        let condition = self.expr()?;
        let body = self.stmt_block()?;
        let source = start.merge(&body);
        Ok(UntypedStmt::new(
            StmtKind::While {
                condition,
                body: Box::new(body),
            },
            source,
        ))
    }

    pub(crate) fn expr(&mut self) -> Result<UntypedExpr> {
        self.expr_bp(0)
    }

    /// Parses the next expression starting with the given minimum binding
    /// power. The binding power is required for the pratt parsing algorithm
    /// that we use.
    fn expr_bp(&mut self, min_bp: u8) -> Result<UntypedExpr> {
        let next = self.advance()?;
        let mut lhs = self.expr_lhs(next)?;
        while self.has_next() {
            let op = self.peek()?.token_type();
            if let Some(l_bp) = Parser::<I>::postfix_binding_power(op) {
                // If current binding power is below minimum, we return the left
                // hand side expression
                if l_bp < min_bp {
                    break;
                }
                let next = self.advance()?;
                match next.token_type() {
                    TokenType::BracketLeft => {
                        lhs = self.expr_index_array(lhs)?;
                    }
                    TokenType::Dot => {
                        lhs = self.expr_access_struct(lhs)?;
                    }
                    TokenType::ParenLeft => {
                        lhs = self.expr_call(lhs)?;
                    }
                    unexpected => unreachable!("Unexpected operator: {:?}", unexpected),
                }
            } else if let Some((l_bp, r_bp)) = Parser::<I>::infix_binding_power(op) {
                // If current binding power is below minimum, we return the left
                // hand side expression
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

    /// Parses the left hand side of a expression.
    fn expr_lhs(&mut self, next: Token) -> Result<UntypedExpr> {
        match next.token_type() {
            TokenType::Bool(_) | TokenType::Int(_) | TokenType::Float(_) | TokenType::String(_) => {
                self.expr_literal(next)
            }
            TokenType::Identifier(_) => self.expr_identifier(next),
            TokenType::VSelf => self.expr_self(next),
            TokenType::Add | TokenType::Sub | TokenType::Not | TokenType::Star | TokenType::Ref => {
                self.expr_unary(next)
            }
            TokenType::ParenLeft => self.expr_grouping(),
            TokenType::BracketLeft => self.expr_create_array(next),
            TokenType::If => self.expr_if(next),
            x => error(diagnostic(
                format!("found {:?} expected expression", x),
                &next,
            ))?,
        }
    }

    /// Parses an unary expression.
    ///
    /// ```txt
    /// unary_op expr
    /// ```
    fn expr_unary(&mut self, op: Token) -> Result<UntypedExpr> {
        let r_bp = Parser::<I>::prefix_binding_power(op.token_type());
        let expr = self.expr_bp(r_bp)?;
        let unary_op = ExprOrigin::try_from(op)?;
        Ok(unary(unary_op, expr, ()))
    }

    /// Parses a binary expression.
    ///
    /// ```txt
    /// expr binary_op expr
    /// ```
    fn expr_binary(&mut self, left: UntypedExpr, bp: u8) -> Result<UntypedExpr> {
        let operator = self.advance()?;
        let right = self.expr_bp(bp)?;
        let operator = ExprOrigin::try_from(operator)?;
        Ok(binary(left, operator, right, ()))
    }

    /// Parses an function call. The callee is the lambda expression, function
    /// or associated method that gets called.
    ///
    /// ```txt
    /// callee "(" (expr ("," expr)*)? ")"
    /// ```
    fn expr_call(&mut self, callee: UntypedExpr) -> Result<UntypedExpr> {
        let mut arguments = Vec::new();
        while *self.peek()?.token_type() != TokenType::ParenRight {
            arguments.push(self.expr()?);
            if *self.peek()?.token_type() != TokenType::ParenRight {
                self.consume(&TokenType::Comma)?;
            }
        }
        let end = self.consume(&TokenType::ParenRight)?;
        let source = callee.merge(&end);
        Ok(call(callee, arguments, source, ()))
    }

    /// Parses an index array operation.
    ///
    /// ```txt
    /// object "[" index "]"
    /// ```
    fn expr_index_array(&mut self, object: UntypedExpr) -> Result<UntypedExpr> {
        let index = self.expr()?;
        let end = self.consume(&TokenType::BracketRight)?;
        let source = object.merge(&end);

        Ok(index_array(object, index, source, ()))
    }

    /// Parses an struct access operation.
    ///
    /// ```txt
    /// left "." right
    /// ```
    fn expr_access_struct(&mut self, left: UntypedExpr) -> Result<UntypedExpr> {
        let right = self.advance()?;
        if !right.token_type().is_identifier() {
            return error(diagnostic("expected an identidier on the RHS", &right));
        }
        let right = Identifier::from(right);
        Ok(access_struct(left, right, ()))
    }

    /// Parses an array creation.
    ///
    /// ```txt
    /// "[" type ";" expr ("," expr)* ","? "]"
    /// ```
    fn expr_create_array(&mut self, start: Token) -> Result<UntypedExpr> {
        let type_ = self.decl_type()?;
        self.consume(&TokenType::Semicolon)?;
        let mut fields = Vec::new();
        while *self.peek()?.token_type() != TokenType::BracketRight {
            fields.push(self.expr()?);
            if *self.peek()?.token_type() != TokenType::BracketRight {
                self.consume(&TokenType::Comma)?;
            }
        }
        let type_ = array(type_, fields.len());
        let end = self.consume(&TokenType::BracketRight)?;

        let source = start.merge(&end);
        Ok(create_array(fields, source, type_))
    }

    fn expr_create_struct(&mut self, struct_name: Token) -> Result<UntypedExpr> {
        let current = self.consume(&TokenType::BraceLeft)?;
        let mut fields = Vec::new();
        while *self.peek()?.token_type() != TokenType::BraceRight {
            let name = self.advance()?;
            if !name.token_type().is_identifier() {
                return error(diagnostic(
                    "expected field name identifier for the struct",
                    &name,
                ));
            }
            self.consume(&TokenType::Set)?;
            let expr = self.expr()?;
            let field = (Identifier::from(name), expr);
            fields.push(field);
            if *self.peek()?.token_type() != TokenType::BraceRight {
                self.consume(&TokenType::Comma)?;
            }
        }
        self.consume(&TokenType::BraceRight)?;
        let source = struct_name.merge(&current);

        // TODO: parse more complex names and generics
        let identifier = Identifier::from(struct_name);
        Ok(create_struct(
            fields,
            source,
            named(identifier.into(), vec![]),
        ))
    }

    fn expr_grouping(&mut self) -> Result<UntypedExpr> {
        // Parse a new expression from the top
        let lhs = self.expr();
        // Consume the ending parenthesis
        self.consume(&TokenType::ParenRight)?;
        lhs
    }

    fn expr_identifier(&mut self, next: Token) -> Result<UntypedExpr> {
        match *self.peek()?.token_type() {
            TokenType::BraceLeft => self.expr_create_struct(next),
            _ => {
                let name = Identifier::from(next);
                Ok(variable(name, ()))
            }
        }
    }

    fn expr_if(&mut self, start: Token) -> Result<UntypedExpr> {
        let condition = self.expr()?;
        self.consume(&TokenType::BraceLeft)?;
        let if_branch = self.stmt()?;
        let end = self.consume(&TokenType::BraceRight)?;
        let (else_branch, source) = if *self.peek()?.token_type() == TokenType::Else {
            self.consume(&TokenType::Else)?;
            self.consume(&TokenType::BraceLeft)?;
            let else_branch = self.stmt()?;
            let end = self.consume(&TokenType::BraceRight)?;
            let source = start.merge(&end);
            (Some(else_branch), source)
        } else {
            let source = start.merge(&end);
            (None, source)
        };
        Ok(if_(condition, if_branch, else_branch, source, ()))
    }

    fn expr_literal(&mut self, next: Token) -> Result<UntypedExpr> {
        Ok(literal(ExprOrigin::try_from(next)?, ()))
    }

    fn expr_self(&mut self, next: Token) -> Result<UntypedExpr> {
        let name = Identifier::from(next);
        Ok(variable(name, ()))
    }

    fn prefix_binding_power(operator: &TokenType) -> u8 {
        match operator {
            TokenType::Not => 1,
            TokenType::Add | TokenType::Sub => 6,
            TokenType::Ref | TokenType::Star => 14,
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
            TokenType::And | TokenType::Or | TokenType::Vbar => Some((3, 4)),
            TokenType::Low
            | TokenType::Leq
            | TokenType::Eq
            | TokenType::Gre
            | TokenType::Geq
            | TokenType::Neq => Some((5, 6)),
            TokenType::Add | TokenType::Sub | TokenType::Mod => Some((7, 8)),
            TokenType::Star | TokenType::Div => Some((9, 10)),
            _ => None,
        }
    }

    fn postfix_binding_power(operator: &TokenType) -> Option<u8> {
        match operator {
            // The BraceRight token type is not included here,
            // because we expect that it should only end a statement.
            TokenType::BracketLeft | TokenType::ParenLeft | TokenType::Dot => Some(11),
            _ => None,
        }
    }

    /// Checks if the next token has the expected token type. If not, throws an
    /// error. Otherwise it consumes and returns the next token.
    fn consume(&mut self, expected: &TokenType) -> Result<Token> {
        let next = self.advance()?;
        if next.token_type() != expected {
            // The consumed token was not of the expected type
            return error(diagnostic(
                format!(
                    "expected {:?} but found {:?}",
                    expected.clone(),
                    next.token_type()
                ),
                &next,
            ));
        }
        Ok(next)
    }

    /// Returns and consumes the next token. Throws an error if no next
    /// token exists or if the underlying token iterator had an error.
    fn advance(&mut self) -> Result<Token> {
        if let Some(c) = self.last.take() {
            return Ok(c);
        }
        // TODO: add better error handling, this was already covered in another
        // pull request
        self.tokens.next().ok_or_else(|| Error::new(vec![]))?
    }

    /// Returns but does not consume the next token. Throws an error if no next
    /// token exists or if the underlying token iterator had an error.
    fn peek(&mut self) -> Result<&Token> {
        if let Some(ref c) = self.last {
            return Ok(c);
        }
        let c = self.advance()?;
        self.last = Some(c);
        self.peek()
    }

    /// Returns true if any tokens are left, otherwise false.
    fn has_next(&mut self) -> bool {
        self.peek().is_ok()
    }
}
