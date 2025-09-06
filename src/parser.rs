use crate::{
    ast::{Decl, Expr, Stmt},
    lexer::{Token, TokenType},
};
use std::collections::VecDeque;
use std::rc::Rc;

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

    pub fn parse(mut self) -> Vec<Decl> {
        while self.has_next() {
            let declaration = self.decl();
            self.declarations.push(declaration);
        }
        self.declarations
    }

    pub(crate) fn decl(&mut self) -> Decl {
        let token = self.advance();
        match token.get_type() {
            TokenType::Import => self.decl_import(),
            TokenType::Function => self.decl_function(),
            TokenType::Struct => self.decl_struct(),
            TokenType::Const => self.decl_const(),
            _ => todo!("unimplemented decl: {:?}", token),
        }
    }

    fn decl_import(&mut self) -> Decl {
        let next = self.advance();
        assert!(next.get_type().is_identifer());
        Decl::Import(next)
    }

    fn decl_function(&mut self) -> Decl {
        let name = self.advance();
        assert!(name.get_type().is_identifer(), "{:?}", name);

        self.consume(&TokenType::ParenLeft);
        let mut params = Vec::new();
        while *self.peek().get_type() != TokenType::ParenRight {
            params.push(self.decl_type_def());
            if *self.peek().get_type() != TokenType::ParenRight {
                self.consume(&TokenType::Comma);
            }
        }
        self.consume(&TokenType::ParenRight);

        self.consume(&TokenType::Colon);
        let return_type = self.advance();
        assert!(name.get_type().is_identifer(), "{:?}", name);

        self.consume(&TokenType::BraceLeft);
        let mut body = Vec::new();
        while *self.peek().get_type() != TokenType::BraceRight {
            body.push(self.stmt());
        }
        self.consume(&TokenType::BraceRight);

        Decl::Function {
            name,
            return_type,
            params,
            body,
        }
    }

    fn decl_struct(&mut self) -> Decl {
        let name = self.advance();
        assert!(name.get_type().is_identifer());
        self.consume(&TokenType::BraceLeft);

        let mut fields = Vec::new();
        while self.peek().get_type().is_identifer() {
            fields.push(self.decl_type_def());
            if *self.peek().get_type() == TokenType::Comma {
                self.consume(&TokenType::Comma);
            }
        }

        let mut methods = Vec::new();
        while *self.peek().get_type() != TokenType::BraceRight {
            self.consume(&TokenType::Function);
            methods.push(Rc::new(self.decl_function()));
        }

        self.consume(&TokenType::BraceRight);
        Decl::Struct {
            name,
            fields,
            methods,
        }
    }

    fn decl_const(&mut self) -> Decl {
        let (name, var_type) = self.decl_type_def();
        self.consume(&TokenType::Set);
        let initializer = self.expr();
        Decl::Const {
            name,
            var_type,
            initializer,
        }
    }

    fn decl_type_def(&mut self) -> (Token, Token) {
        let name = self.advance();
        assert!(name.get_type().is_identifer(), "{:?}", name);
        self.consume(&TokenType::Colon);
        let type_name = self.advance();
        assert!(type_name.get_type().is_identifer(), "{:?}", type_name);
        (name, type_name)
    }

    pub(crate) fn stmt(&mut self) -> Stmt {
        match self.peek().get_type() {
            TokenType::BraceLeft => self.stmt_block(),
            TokenType::Let => self.stmt_let(),
            TokenType::Return => {
                self.advance();
                Stmt::Return { value: self.expr() }
            }
            TokenType::Break => {
                self.advance();
                Stmt::Break
            }
            TokenType::While => self.stmt_while(),
            _ => Stmt::ExprStmt(self.expr()),
        }
    }

    fn stmt_block(&mut self) -> Stmt {
        self.advance();
        let mut statements = Vec::new();
        while *self.peek().get_type() != TokenType::BraceRight {
            statements.push(Rc::new(self.stmt()));
        }
        self.consume(&TokenType::BraceRight);
        Stmt::Block { statements }
    }

    fn stmt_let(&mut self) -> Stmt {
        self.advance();
        let name = self.advance();
        assert!(name.get_type().is_identifer());
        self.consume(&TokenType::Set);
        let initializer = self.expr();
        Stmt::Let { name, initializer }
    }

    fn stmt_while(&mut self) -> Stmt {
        self.advance();
        self.consume(&TokenType::ParenLeft);
        let condition = self.expr();
        self.consume(&TokenType::ParenRight);
        let body = Rc::new(self.stmt());
        Stmt::While { condition, body }
    }

    pub(crate) fn expr(&mut self) -> Expr {
        self.expr_bp(0)
    }

    fn expr_bp(&mut self, min_bp: u8) -> Expr {
        let next = self.advance();
        let mut lhs = match next.get_type() {
            TokenType::Bool(_) | TokenType::Number(_) | TokenType::String(_) => Expr::Literal(next),
            TokenType::Identifier(_) => {
                if *self.peek().get_type() == TokenType::BraceLeft {
                    self.expr_create(next)
                } else {
                    Expr::Variable(next)
                }
            }
            TokenType::VSelf => Expr::Variable(next),
            TokenType::Add | TokenType::Sub | TokenType::Not => self.expr_unary(next),
            TokenType::ParenLeft => self.expr_grouping(),
            TokenType::If => self.expr_if(),
            _ => todo!("unexpected token in expression: {:?}", next),
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
                        lhs = self.expr_index(lhs);
                    }
                    TokenType::ParenLeft => {
                        lhs = self.expr_call(lhs);
                    }
                    unexpected => unreachable!("Unexpected operator: {:?}", unexpected),
                }
            } else if let Some((l_bp, r_bp)) = Parser::infix_binding_power(op) {
                // If current binding power is below minimum, we return the left hand side expression
                if l_bp < min_bp {
                    break;
                }
                lhs = self.expr_binary(lhs, r_bp);
            } else {
                // We reached a token that is not an operator and stop
                break;
            }
        }

        lhs
    }

    fn expr_grouping(&mut self) -> Expr {
        // Parse a new expression from the top
        let lhs = self.expr();
        // Consume the ending parenthesis
        self.consume(&TokenType::ParenRight);
        lhs
    }

    fn expr_index(&mut self, lhs: Expr) -> Expr {
        let rhs = self.expr();
        self.consume(&TokenType::BracketRight);
        Expr::Index {
            object: Rc::new(lhs),
            index: Rc::new(rhs),
        }
    }

    fn expr_call(&mut self, lhs: Expr) -> Expr {
        let mut arguments = Vec::new();
        while *self.peek().get_type() != TokenType::ParenRight {
            arguments.push(Rc::new(self.expr()));
            if *self.peek().get_type() == TokenType::Comma {
                self.consume(&TokenType::Comma);
            }
        }
        self.consume(&TokenType::ParenRight);
        Expr::Call {
            callee: Rc::new(lhs),
            arguments,
        }
    }

    fn expr_unary(&mut self, op: Token) -> Expr {
        let r_bp = Parser::prefix_binding_power(op.get_type());
        let rhs = self.expr_bp(r_bp);
        Expr::Unary {
            operator: op,
            right: Rc::new(rhs),
        }
    }

    fn expr_binary(&mut self, lhs: Expr, bp: u8) -> Expr {
        let next = self.advance();
        if *next.get_type() == TokenType::Dot {
            let rhs = self.advance();
            assert!(rhs.get_type().is_identifer());
            Expr::Get {
                left: Rc::new(lhs),
                right: rhs,
            }
        } else {
            let rhs = self.expr_bp(bp);
            Expr::Binary {
                left: Rc::new(lhs),
                operator: next,
                right: Rc::new(rhs),
            }
        }
    }

    fn expr_create(&mut self, struct_name: Token) -> Expr {
        self.consume(&TokenType::BraceLeft);
        let mut fields = Vec::new();
        while *self.peek().get_type() != TokenType::BraceRight {
            let name = self.advance();
            assert!(name.get_type().is_identifer(), "{:?}", name);
            self.consume(&TokenType::Set);
            let expr = Rc::new(self.expr());
            let field = (name, expr);
            fields.push(field);
            if *self.peek().get_type() != TokenType::BraceRight {
                self.consume(&TokenType::Comma);
            }
        }
        self.consume(&TokenType::BraceRight);
        Expr::Create {
            struct_name,
            fields,
        }
    }

    fn expr_if(&mut self) -> Expr {
        self.consume(&TokenType::ParenLeft);
        let condition = Rc::new(self.expr());
        self.consume(&TokenType::ParenRight);
        let if_branch = Rc::new(self.stmt());
        let else_branch = if *self.peek().get_type() == TokenType::Else {
            self.consume(&TokenType::Else);
            Some(Rc::new(self.stmt()))
        } else {
            None
        };
        Expr::If {
            condition,
            if_branch,
            else_branch,
        }
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

    fn consume(&mut self, expected: &TokenType) -> Token {
        let next = self.advance();
        assert_eq!(next.get_type(), expected, "{:?}", next);
        next
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
