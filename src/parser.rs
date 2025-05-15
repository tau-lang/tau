use crate::ast::{self, Ast, Id, PrimitiveTypes, Type};
use pest::iterators::Pair;

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "../grammar/tau.pest"]
pub struct TauParser;

fn type_name(str: &str) -> PrimitiveTypes {
    match str {
        "f32" => PrimitiveTypes::F32,
        "f64" => PrimitiveTypes::F64,
        "i64" => PrimitiveTypes::I64,
        "i32" => PrimitiveTypes::I32,
        "string" => PrimitiveTypes::String,
        "void" => PrimitiveTypes::Unit,
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
        r @ _ => panic!("expected a typdef, found {:?} in {:?}", r, pair),
    }
}
pub fn parse_tau(pair: Pair<Rule>) -> Ast {
    match pair.as_rule() {
        Rule::root => Ast::Block {
            terms: pair
                .into_inner()
                .map(|pair: Pair<Rule>| parse_tau(pair))
                .collect(),
        },
        Rule::modificationStatement => {
            let mut inner = pair.into_inner();
            // modificationStatement = { variable ~ ((assignOp ~ valueExpr) | unaryInc | unaryDec) }
            let name = inner.next().unwrap().as_span().as_str();
            let action = {
                let i = inner.next().unwrap();
                match i.as_rule() {
                    Rule::unaryInc => Ast::BinaryOp {
                        op: ast::BinaryOp::Add,
                        lhs: ast::Ast::Id(Id::new(name)).r(),
                        rhs: Ast::Primitive(ast::Primitive::Int(1)).r(),
                    },
                    Rule::unaryDec => Ast::BinaryOp {
                        op: ast::BinaryOp::Subtract,
                        lhs: ast::Ast::Id(Id::new(name)).r(),
                        rhs: Ast::Primitive(ast::Primitive::Int(1)).r(),
                    },
                    Rule::assignOp => {
                        let mut i = i.into_inner();
                        let rhs = parse_tau(inner.next().unwrap()).r();
                        match i.next().unwrap().as_rule() {
                            Rule::mulAssign => Ast::BinaryOp {
                                op: ast::BinaryOp::Multiply,
                                lhs: ast::Ast::Id(Id::new(name)).r(),
                                rhs,
                            },
                            Rule::addAssign => Ast::BinaryOp {
                                op: ast::BinaryOp::Add,
                                lhs: ast::Ast::Id(Id::new(name)).r(),
                                rhs,
                            },
                            r @ _ => {
                                panic!("expected one of mulAssign or addAssign but found: {:?}", r)
                            }
                        }
                    }
                    r @ _ => panic!(
                        "expected one of unaryInc, unaryDec or assignOP but found: {:?}",
                        r
                    ),
                }
            };
            Ast::Modification {
                what: Id::new(name),
                val: action.r(),
            }
        }
        Rule::imports => Ast::Imports(
            pair.into_inner()
                .map(|pair| {
                    pair.into_inner()
                        .next()
                        .unwrap()
                        .as_span()
                        .as_str()
                        .to_string()
                })
                .collect(),
        ),
        Rule::typeDef => panic!("should not encounter raw typeDef in AST parsing"),
        Rule::structDecl => {
            let mut i = pair.clone().into_inner();
            Ast::Composit {
                name: i.next().unwrap().as_span().as_str().to_string(),
                fields: i.map(|pair| typedef(pair)).collect::<Vec<Type>>(),
            }
        }
        Rule::declarations | Rule::body => Ast::Block {
            terms: pair.into_inner().map(|pair| parse_tau(pair)).collect(),
        },
        // use if only one is expected to avoid too deep nesting with blocks
        Rule::numberExpr | Rule::declaration | Rule::valueExpr | Rule::statement => {
            parse_tau(pair.into_inner().next().unwrap())
        }
        Rule::functionDecl => {
            let mut inner = pair.into_inner();
            let name = Id::new(inner.next().unwrap().as_span().as_str());
            let len = inner.len();
            let mut args = vec![];
            for _ in 0..(len - 2) {
                args.push(typedef(inner.next().unwrap()));
            }
            let ret_type = type_name(inner.next().unwrap().as_span().as_str());
            Ast::Function {
                name,
                parameters: args,
                body: inner
                    .next()
                    .unwrap()
                    .into_inner()
                    .map(|pair| parse_tau(pair))
                    .collect(),

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
            Ast::Var {
                name: n,
                value: parse_tau(inner.next().unwrap()).r(),
                r#type: t,
            }
        }

        Rule::numberValue => {
            let inner = pair.into_inner().last().unwrap();
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

#[cfg(test)]
mod test {
    use crate::ast::Primitive;

    use super::*;
    #[test]
    fn composit_type() {
        let src = "
struct vec2 {
   x: f64,
   y: i32
}
            ";
        let mut tokens = TauParser::parse(Rule::root, src).unwrap_or_else(|e| panic!("{}", e));
        assert_eq!(
            parse_tau(tokens.next().unwrap()),
            Ast::Block {
                terms: vec![
                    Ast::Imports(vec![]),
                    Ast::Block {
                        terms: vec![Ast::Composit {
                            name: "vec2".to_string(),
                            fields: vec![
                                Type {
                                    name: "x".to_string(),
                                    r#type: PrimitiveTypes::F64
                                },
                                Type {
                                    name: "y".to_string(),
                                    r#type: PrimitiveTypes::I32
                                }
                            ]
                        }],
                    },
                    Ast::Block { terms: vec![] }
                ]
            }
        )
    }
    #[test]
    fn imports() {
        let src = "
import math
import mymod
            ";
        let mut tokens = TauParser::parse(Rule::root, src).unwrap_or_else(|e| panic!("{}", e));
        assert_eq!(
            parse_tau(tokens.next().unwrap()),
            Ast::Block {
                terms: vec![
                    Ast::Imports(vec!["math".to_string(), "mymod".to_string()]),
                    Ast::Block { terms: vec![] },
                    Ast::Block { terms: vec![] }
                ]
            }
        )
    }
    #[test]
    fn mul_assign() {
        let src = "

fn example(x: i32): void {
    x*=2
}
            ";
        let mut tokens = TauParser::parse(Rule::root, src).unwrap_or_else(|e| panic!("{}", e));
        assert_eq!(
            parse_tau(tokens.next().unwrap()),
            Ast::Block {
                terms: vec![
                    Ast::Imports(vec![]),
                    Ast::Block {
                        terms: vec![Ast::Function {
                            name: Id::new("example"),
                            return_type: PrimitiveTypes::Unit,
                            parameters: vec![Type {
                                name: "x".to_string(),
                                r#type: PrimitiveTypes::I32
                            }],
                            body: vec![Ast::Modification {
                                what: Id::new("x"),
                                val: Ast::BinaryOp {
                                    op: ast::BinaryOp::Multiply,
                                    lhs: Ast::Id(Id::new("x")).r(),
                                    rhs: Ast::Primitive(Primitive::Int(2)).r(),
                                }
                                .r()
                            }]
                        }]
                    },
                    Ast::Block { terms: vec![] }
                ]
            }
        )
    }
    #[test]
    fn function() {
        let src = "

fn example(x: i32): void {
}
            ";
        let mut tokens = TauParser::parse(Rule::root, src).unwrap_or_else(|e| panic!("{}", e));
        assert_eq!(
            parse_tau(tokens.next().unwrap()),
            Ast::Block {
                terms: vec![
                    Ast::Imports(vec![]),
                    Ast::Block {
                        terms: vec![Ast::Function {
                            name: Id::new("example"),
                            return_type: PrimitiveTypes::Unit,
                            parameters: vec![Type {
                                name: "x".to_string(),
                                r#type: PrimitiveTypes::I32
                            }],
                            body: vec![]
                        }]
                    },
                    Ast::Block { terms: vec![] }
                ]
            }
        )
    }
}
