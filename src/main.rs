pub mod ast;

use ast::{Ast, Id, PrimitiveTypes, Type};

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "../grammar/tau.pest"]
struct TauParser;

fn main() {
    let pairs = TauParser::parse(Rule::root, include_str!("../examples/vec2.tau"))
        .unwrap_or_else(|e| panic!("{}", e));
    use pest::iterators::Pair;
    dbg!(pairs.map(|pair| parse_tau(pair)).collect::<Vec<Ast>>());
    fn type_name(str: &str) -> PrimitiveTypes {
        match str {
            "f32" => PrimitiveTypes::F32,
            "f64" => PrimitiveTypes::F64,
            "i64" => PrimitiveTypes::I64,
            "i32" => PrimitiveTypes::I32,
            "string" => PrimitiveTypes::String,
            c @ _ => PrimitiveTypes::Custom(c.to_string()),
        }
    }
    fn typedef(pair: Pair<Rule>) -> Type {
        match pair.as_rule() {
            Rule::typeDef => {
                let mut inner = pair.into_inner();
                let n = inner.next().unwrap().as_span().as_str().to_string();
                let t = type_name(inner.last().unwrap().as_span().as_str());
                Type { name: n, r#type: t }
            }
            r @ _ => panic!("expected a typdef, found {:?}", r),
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
            Rule::imports => Ast::Id(Id::new("this is where the imports will be")),
            Rule::typeDef => panic!("should not encounter raw typeDef in AST parsing"),
            Rule::structDecl => {
                let i = pair.clone().into_inner();
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
            Rule::declaration
            | Rule::declarations
            | Rule::body
            | Rule::statement
            | Rule::valueExpr => Ast::Block {
                terms: pair.into_inner().map(|pair| parse_tau(pair)).collect(),
            },

            // use if only one is expected to avoid too deep nesting with blocks
            Rule::numberExpr => parse_tau(pair.into_inner().next().unwrap()),
            Rule::functionDecl => {
                let mut inner = pair.into_inner();
                let name = Id::new(inner.next().unwrap().as_span().as_str());
                let len = inner.len();
                let mut args = vec![];
                for _ in 0..(len - 2) {
                    args.push(typedef(inner.next().unwrap()));
                }
                let ret_type = type_name(inner.next().unwrap().as_span().as_str());
                let body = parse_tau(inner.next().unwrap());
                Ast::Function {
                    name,
                    parameters: args,
                    body: body.r(),
                    return_type: ret_type,
                }
            }
            Rule::EOI => Ast::Block { terms: vec![] },
            Rule::variableStatement => {
                let mut inner = pair.into_inner();
                let (n, t) = {
                    let mut decl = inner.next().unwrap().into_inner();
                    let n = decl.next().unwrap().as_span().as_str().to_string();
                    (n, type_name(decl.next().unwrap().as_span().as_str()))
                };
                dbg!(Ast::Var {
                    name: n,
                    value: parse_tau(inner.next().unwrap()).r(),
                    r#type: t,
                })
            }

            Rule::numberValue => {
                let inner = dbg!(pair.into_inner().last().unwrap());
                if let Some(typ) = inner.clone().into_inner().next() {
                    match typ.as_rule() {
                        Rule::integer => Ast::Primitive(ast::Primitive::Int(
                            inner.as_span().as_str().parse().unwrap(),
                        )),
                        Rule::float => Ast::Primitive(ast::Primitive::Float(
                            inner.as_span().as_str().parse().unwrap(),
                        )),
                        r @ _ => panic!("expected int or float, got {:?}", r),
                    }
                } else {
                    Ast::Id(Id::new(inner.as_span().as_str()))
                }
            }
            Rule::returnStatement => Ast::Return {
                term: Ast::Block {
                    terms: pair.into_inner().map(|pair| parse_tau(pair)).collect(),
                }
                .r(),
            },
            e @ _ => todo!("{:?}", e),
        }
    }
}
