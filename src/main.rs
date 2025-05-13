pub mod ast;

use ast::{Ast, Id, PrimitiveTypes, Type};

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
    fn typedef(pair: Pair<Rule>) -> Type {
        match pair.as_rule() {
            Rule::typeDef => {
                let mut inner = pair.into_inner();
                let n = inner.next().unwrap().as_span().as_str().to_string();
                let t = match inner.last().unwrap().as_span().as_str() {
                    "f32" => PrimitiveTypes::F32,
                    "f64" => PrimitiveTypes::F64,
                    "i64" => PrimitiveTypes::I64,
                    "i32" => PrimitiveTypes::I32,
                    "string" => PrimitiveTypes::String,
                    c @ _ => PrimitiveTypes::Custom(c.to_string()),
                };
                Type { name: n, r#type: t }
            }
            _ => panic!("expected a typdef"),
        }
    }
    fn parse_tau(pair: Pair<Rule>) -> Ast {
        match pair.as_rule() {
            Rule::root => Ast::Block {
                terms: pair
                    .into_inner()
                    .map(|pair: Pair<Rule>| parse_tau(pair))
                    .collect(),
            },
            Rule::imports => Ast::Block { terms: vec![] },
            Rule::typeDef => panic!("should not encounter raw typeDef in AST parsing"),
            Rule::structDecl => {
                let mut i = pair.clone().into_inner();
                i.next();
                Ast::Composit {
                    name: pair
                        .clone()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_span()
                        .as_str()
                        .to_string(),
                    fields: i.map(|pair| typedef(pair)).collect::<Vec<Type>>(),
                }
            }
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
