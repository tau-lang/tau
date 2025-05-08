pub mod ast;

use ast::{Ast, Id};

fn main() {
    let a = Ast::Block {
        terms: vec![
            Ast::Function {
                name: Id::new("pow"),
                parameters: vec!["x".to_string()],
                body: Ast::Multiply {
                    lhs: Ast::Id(Id::new("x")).r(),
                    rhs: Ast::Id(Id::new("x")).r(),
                }
                .r(),
            },
            Ast::Var {
                name: "x".into(),
                value: Ast::Multiply {
                    lhs: Ast::Number(12).r(),
                    rhs: Ast::Add {
                        lhs: Ast::Number(12).r(),
                        rhs: Ast::Number(145242).r(),
                    }
                    .r(),
                }
                .r(),
            },
            Ast::Call {
                callee: Id::new("pow"),
                args: vec![Ast::Id(Id::new("x"))],
            },
        ],
    }
    .r();
    dbg!(&a);
}
