use super::Rule;
use super::error;
use super::error::Expected;
use super::error::expected_pair;
use crate::ast::{self, Ast, Id, PrimitiveTypes, Type};
use miette::SourceOffset;
use pest::iterators::Pair;

fn type_name(str: &str) -> PrimitiveTypes {
    match str {
        "f32" => PrimitiveTypes::F32,
        "f64" => PrimitiveTypes::F64,
        "i64" => PrimitiveTypes::I64,
        "i32" => PrimitiveTypes::I32,
        "string" => PrimitiveTypes::String,
        "void" => PrimitiveTypes::Unit,
        c => PrimitiveTypes::Custom(c.to_string()),
    }
}
fn typedef(pair: Pair<Rule>) -> Result<Type, error::Source> {
    match pair.as_rule() {
        Rule::typeDef => {
            let mut inner = pair.clone().into_inner();
            let n = inner
                .next()
                .ok_or_else(|| expected_pair(Expected::Name, &pair))?
                .as_span()
                .as_str()
                .to_string();
            let t = type_name(
                inner
                    .next_back()
                    .ok_or(expected_pair(Expected::Type, &pair))?
                    .as_span()
                    .as_str(),
            );
            Ok(Type { name: n, r#type: t })
        }
        r => Err(expected_pair(Expected::Found(Rule::typeDef, r), &pair)),
    }
}
fn struct_init_type(pair: Pair<Rule>) -> Result<Vec<(String, Ast)>, error::Source> {
    pair.clone()
        .into_inner()
        .map(|p| match p.as_rule() {
            Rule::tablePair => {
                let mut i = p.into_inner();
                let name = i
                    .next()
                    .ok_or(expected_pair(Expected::Name, &pair))?
                    .as_span()
                    .as_str()
                    .to_string();
                match i
                    .next()
                    .ok_or(expected_pair(Expected::Ast, &pair))
                    .and_then(parse_tau)
                {
                    Ok(v) => Ok((name, v)),
                    Err(e) => Err(e),
                }
            }
            r => Err(expected_pair(Expected::Found(Rule::tablePair, r), &pair)),
        })
        .collect::<Result<Vec<(String, Ast)>, error::Source>>()
}

pub fn parse_tau(pair: Pair<Rule>) -> Result<Ast, super::error::Source> {
    Ok(match pair.as_rule() {
        Rule::root => Ast::Block {
            terms: pair
                .into_inner()
                .map(|pair: Pair<Rule>| parse_tau(pair))
                .collect::<Result<Vec<_>, error::Source>>()?,
        },
        Rule::tableExpr => Ast::CompositConstruction {
            values: struct_init_type(pair)?,
        },
        Rule::modificationStatement => {
            let mut inner = pair.clone().into_inner();
            let name = inner
                .next()
                .ok_or(expected_pair(Expected::Name, &pair))?
                .as_span()
                .as_str();
            let action = {
                let i = inner.next().ok_or(expected_pair(Expected::Name, &pair))?;
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
                        let rhs =
                            parse_tau(inner.next().ok_or(expected_pair(Expected::Ast, &pair))?)?
                                .r();
                        match i
                            .next()
                            .ok_or(expected_pair(Expected::Ast, &pair))?
                            .as_rule()
                        {
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
                            Rule::subAssign => Ast::BinaryOp {
                                op: ast::BinaryOp::Subtract,
                                lhs: ast::Ast::Id(Id::new(name)).r(),
                                rhs,
                            },
                            Rule::divAssign => Ast::BinaryOp {
                                op: ast::BinaryOp::Divide,
                                lhs: ast::Ast::Id(Id::new(name)).r(),
                                rhs,
                            },
                            r => {
                                panic!(
                                    "expected one of mulAssign, divAssign, subAssign or addAssign but found: {:?}",
                                    r
                                )
                            }
                        }
                    }
                    r => panic!(
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
                    pair.clone()
                        .into_inner()
                        .next()
                        .ok_or(expected_pair(Expected::Import, &pair))
                        .map(|p| p.as_span().as_str().to_string())
                })
                .collect::<Result<Vec<_>, error::Source>>()?,
        ),
        Rule::typeDef => panic!("should not encounter raw typeDef in AST parsing"),
        Rule::structDecl => {
            let mut i = pair.clone().into_inner();
            Ast::CompositDef {
                name: i
                    .next()
                    .ok_or(expected_pair(Expected::Name, &pair))?
                    .as_span()
                    .as_str()
                    .to_string(),
                fields: i
                    .map(|pair| typedef(pair))
                    .collect::<Result<Vec<Type>, error::Source>>()?,
            }
        }
        Rule::declarations | Rule::body => Ast::Block {
            terms: pair
                .into_inner()
                .map(|pair| parse_tau(pair))
                .collect::<Result<Vec<_>, error::Source>>()?,
        },
        // use if only one is expected to avoid too deep nesting with blocks
        Rule::numberExpr | Rule::declaration | Rule::valueExpr | Rule::statement => parse_tau(
            pair.clone()
                .into_inner()
                .next()
                .ok_or(expected_pair(Expected::Ast, &pair))?,
        )?,
        Rule::functionDecl => {
            let mut inner = pair.clone().into_inner();
            let name = Id::new(
                inner
                    .next()
                    .ok_or(expected_pair(Expected::Name, &pair))?
                    .as_span()
                    .as_str(),
            );
            let len = inner.len();
            let mut args = vec![];
            for _ in 0..(len - 2) {
                args.push(typedef(
                    inner.next().ok_or(expected_pair(Expected::Type, &pair))?,
                )?);
            }
            let ret_type = type_name(
                inner
                    .next()
                    .ok_or(expected_pair(Expected::Type, &pair))?
                    .as_span()
                    .as_str(),
            );
            Ast::Function {
                name,
                parameters: args,
                body: inner
                    .next()
                    .ok_or(expected_pair(Expected::Ast, &pair))?
                    .into_inner()
                    .map(|pair| parse_tau(pair))
                    .collect::<Result<Vec<_>, error::Source>>()?,

                return_type: ret_type,
            }
        }
        Rule::EOI => Ast::Block { terms: vec![] },
        Rule::variableStatement => {
            let mut inner = pair.clone().into_inner();
            let (n, t) = {
                let mut decl = inner
                    .next()
                    .ok_or(expected_pair(Expected::Name, &pair))?
                    .into_inner();
                let n = decl
                    .next()
                    .ok_or(expected_pair(Expected::Name, &pair))?
                    .as_span()
                    .as_str()
                    .to_string();
                (
                    n,
                    type_name(
                        decl.next()
                            .ok_or(expected_pair(Expected::Type, &pair))?
                            .as_span()
                            .as_str(),
                    ),
                )
            };
            Ast::Var {
                name: n,
                value: parse_tau(inner.next().ok_or(expected_pair(Expected::Ast, &pair))?)?.r(),
                r#type: t,
            }
        }

        Rule::numberValue => {
            let inner = pair
                .clone()
                .into_inner()
                .next_back()
                .ok_or(expected_pair(Expected::Ast, &pair))?;
            if let Some(typ) = inner.clone().into_inner().next() {
                match typ.as_rule() {
                    Rule::integer => Ast::Primitive(ast::Primitive::Int(
                        inner
                            .as_span()
                            .as_str()
                            .parse()
                            .map_err(|e| expected_pair(Expected::Int(e), &pair))?,
                    )),
                    Rule::float => Ast::Primitive(ast::Primitive::Float(
                        inner
                            .as_span()
                            .as_str()
                            .parse()
                            .map_err(|e| expected_pair(Expected::Float(e), &pair))?,
                    )),
                    r => panic!("expected int or float, got {:?}", r),
                }
            } else {
                Ast::Id(Id::new(inner.as_span().as_str()))
            }
        }
        Rule::returnStatement => Ast::Return {
            term: Ast::Block {
                terms: pair
                    .into_inner()
                    .map(|pair| parse_tau(pair))
                    .collect::<Result<Vec<_>, error::Source>>()?,
            }
            .r(),
        },
        e => todo!("{:?}", e),
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ast::Primitive;
    use crate::parser::{Parser, TauParser};
    #[test]
    fn composit_type() {
        let src = "
    struct vec2 {
       x: f64,
       y: i32
    }
                ";
        let ast = TauParser::parse(src);
        assert_eq!(
            ast,
            Ok(Ast::Block {
                terms: vec![
                    Ast::Imports(vec![]),
                    Ast::Block {
                        terms: vec![Ast::CompositDef {
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
            })
        )
    }
    #[test]
    fn imports() {
        let src = "
    import math
    import mymod
                ";
        let ast = TauParser::parse(src);
        assert_eq!(
            ast,
            Ok(Ast::Block {
                terms: vec![
                    Ast::Imports(vec!["math".to_string(), "mymod".to_string()]),
                    Ast::Block { terms: vec![] },
                    Ast::Block { terms: vec![] }
                ]
            })
        )
    }
    #[test]
    fn mul_assign() {
        let src = "

    fn example(x: i32): void {
        x*=2
    }
                ";
        let ast = TauParser::parse(src);
        assert_eq!(
            ast,
            Ok(Ast::Block {
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
            })
        )
    }
    #[test]
    fn example() {
        assert!(TauParser::parse(include_str!("../../examples/vec2.tau")).is_ok())
    }
    #[test]
    fn function() {
        let src = "

fn example(x: i32): void {
}
            ";
        let ast = TauParser::parse(src);
        assert_eq!(
            ast,
            Ok(Ast::Block {
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
            })
        )
    }
}
