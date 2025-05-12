pub mod ast;

use ast::{Ast, Id};

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "../grammar/tau.pest"]
struct TauParser;

fn main() {
    let mut pairs = TauParser::parse(Rule::root, include_str!("../examples/vec2.tau"))
        .unwrap_or_else(|e| panic!("{}", e));
    use pest::iterators::Pair;
    dbg!(parse_tau(pairs.next().unwrap()));
    fn parse_tau(pair: Pair<Rule>) -> Ast {
        match pair.as_rule() {
            Rule::root => Ast::Block {
                terms: pair
                    .into_inner()
                    .map(|pair: Pair<Rule>| parse_tau(pair))
                    .collect(),
            },
            Rule::imports => Ast::Block { terms: vec![] },
            Rule::structDecl => Ast::Composit(ast::Type::Primitive((
                "vec2".to_string(),
                ast::PrimitiveTypes::Number,
            ))),
            Rule::declaration | Rule::declarations => parse_tau(pair.into_inner().next().unwrap()),
            Rule::functionDecl => {
                // dbg!(&pair);
                Ast::Id(Id::new(pair.as_str()))
                // Ast::Function {
                //     name: Id::new(&pair.as_str()),
                //     parameters: todo!(),
                //     body: todo!(),
                // }
            }
            Rule::EOI => Ast::Block { terms: vec![] },
            // Rule::functionDecl => Ast::Function {
            //     name: (),
            //     parameters: (),
            //     body: (),
            // },
            e @ _ => todo!("{:?}", e),
        }
    }
}
