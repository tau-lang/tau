use crate::{
    ast::Expr,
    lexer::{Token, TokenType},
};
use std::collections::VecDeque;
use std::rc::Rc;

// Pratt parser inspired after the following block post:
// https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html
pub struct Parser {
    tokens: VecDeque<Token>,
}

impl Parser {
    pub fn new(tokens: VecDeque<Token>) -> Parser {
        Parser { tokens }
    }

    pub fn parse(mut self) -> Expr {
        self.expr()
    }

    fn expr(&mut self) -> Expr {
        self.expr_bp(0)
    }

    fn expr_bp(&mut self, min_bp: u8) -> Expr {
        let next = self.tokens.pop_front().expect("Expected next token.");
        let mut lhs = match next.get_type() {
            TokenType::Number(_) | TokenType::String(_) => Expr::Literal(next),
            TokenType::Identifier(name) => Expr::Variable(name.clone()),
            TokenType::Add | TokenType::Sub | TokenType::Not => self.expr_unary(next),
            TokenType::ParenLeft => self.expr_grouping(),
            _ => todo!("{:?}", next.get_type()),
        };

        while self.has_next() {
            let op = self.peek().get_type();
            if let Some(l_bp) = Parser::postfix_binding_power(op) {
                // If current binding power is below minimum, we return the left hand side expression
                if l_bp < min_bp {
                    break;
                }
                let next = self.tokens.pop_front().unwrap();
                match next.get_type() {
                    TokenType::BracketLeft => {
                        lhs = self.expr_index(lhs);
                    }
                    TokenType::ParenLeft => {
                        lhs = self.expr_call(lhs);
                    }
                    unexpected => unreachable!("Unexpected postfix operator: {:?}", unexpected),
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
        let next = self.tokens.pop_front().unwrap();
        let rhs = self.expr_bp(bp);
        Expr::Binary {
            left: Rc::new(lhs),
            operator: next,
            right: Rc::new(rhs),
        }
    }

    fn prefix_binding_power(operator: &TokenType) -> u8 {
        match operator {
            TokenType::Add | TokenType::Sub => 6,
            _ => panic!("Bad operator: {:?}", operator),
        }
    }

    fn infix_binding_power(operator: &TokenType) -> Option<(u8, u8)> {
        match operator {
            TokenType::And | TokenType::Or | TokenType::Xor => Some((1, 2)),
            TokenType::Add | TokenType::Sub => Some((2, 3)),
            TokenType::Mul | TokenType::Div => Some((4, 5)),
            TokenType::Dot => Some((8, 7)),
            _ => None,
        }
    }

    fn postfix_binding_power(operator: &TokenType) -> Option<u8> {
        match operator {
            TokenType::BracketLeft | TokenType::ParenLeft => Some(7),
            _ => None,
        }
    }

    fn consume(&mut self, expected: &TokenType) {
        assert_eq!(self.tokens.pop_front().unwrap().get_type(), expected);
    }

    fn peek(&self) -> &Token {
        self.tokens.front().unwrap()
    }

    fn has_next(&self) -> bool {
        self.tokens.len() > 1
    }
}
