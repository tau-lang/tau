use crate::{
    analyser::SemanticAnalyser,
    ast::{
        Identifier, Location, TypedExpr,
        expression::{Expr, ExprFolder, ExprKind},
        origin::{BinaryOp, ExprOrigin, Literal, UnaryOp},
        prelude::*,
        statement::{Stmt, StmtFolder, StmtKind},
    },
    error::{Error, Result, diagnostic, error, label, primary},
    lexer::Source,
    type_::{Type, Typed, Untyped, collapse_links, prelude::*, unify},
};
use std::boxed::Box;

impl ExprFolder for SemanticAnalyser {
    type Type = Untyped;
    type Output = Result<TypedExpr>;

    fn fold_unary(
        &mut self,
        operator: ExprOrigin<UnaryOp>,
        right: Expr<Self::Type>,
        _: Self::Type,
    ) -> Self::Output {
        let right = self.fold_expr(right)?;
        match operator.kind() {
            UnaryOp::Neg => {
                if !right.type_().is_number() {
                    return error(
                        diagnostic(
                            format!("expected number, got {}", right.type_()),
                            operator.merge(&right),
                        )
                        .with_label(label(format!("right has type {}", right.type_()), &right)),
                    );
                }
                let type_ = right.type_().clone();
                Ok(unary(operator, right, type_))
            }
            UnaryOp::Not => {
                unify(right.type_().clone(), bool()).map_err(|d| {
                    Error::with_diagnostic(d.with_label(primary(operator.merge(&right))))
                })?;
                Ok(unary(operator, right, bool()))
            }
            UnaryOp::Ref => {
                let type_ = ptr(right.type_().clone(), true);
                Ok(unary(operator, right, type_))
            }
            UnaryOp::Deref => match right.type_().as_ref() {
                Type::Ptr { type_, .. } => {
                    let type_ = type_.clone();
                    Ok(unary(operator, right, type_))
                }
                _ => error(
                    diagnostic(
                        format!("expected pointer, got {}", right.type_()),
                        operator.merge(&right),
                    )
                    .with_label(label(format!("right has type {}", right.type_()), &right)),
                ),
            },
        }
    }

    fn fold_binary(
        &mut self,
        left: Expr<Self::Type>,
        operator: ExprOrigin<BinaryOp>,
        right: Expr<Self::Type>,
        _: Self::Type,
    ) -> Self::Output {
        Ok(match operator.kind() {
            // (a -> a) -> a
            BinaryOp::Set => {
                let left = self.infer(left)?;
                let right = self.infer(right)?;
                unify(left.type_().clone(), right.type_().clone()).map_err(|d| {
                    Error::with_diagnostic(
                        d.with_label(primary(left.merge(&right)))
                            .with_label(label(
                                format!("left side has type {}", left.type_()),
                                &left,
                            ))
                            .with_label(label(
                                format!("right side has type {}", right.type_()),
                                &right,
                            )),
                    )
                })?;
                let type_ = left.type_().clone();
                binary(left, operator, right, type_)
            }

            // (number -> number) -> number
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::SetAdd
            | BinaryOp::SetSub
            | BinaryOp::SetMul
            | BinaryOp::SetDiv => {
                let left = self.infer(left)?;
                let right = self.infer(right)?;
                if !left.type_().is_number() || !right.type_().is_number() {
                    return error(
                        diagnostic("left and right side must be numbers", left.merge(&right))
                            .with_label(label(
                                format!("left side has type {}", left.type_()),
                                &left,
                            ))
                            .with_label(label(
                                format!("right side has type {}", right.type_()),
                                &right,
                            )),
                    );
                }
                unify(left.type_().clone(), right.type_().clone()).map_err(|d| {
                    Error::with_diagnostic(
                        d.with_label(primary(left.merge(&right)))
                            .with_label(label(
                                format!("left side has type {}", left.type_()),
                                &left,
                            ))
                            .with_label(label(
                                format!("right side has type {}", right.type_()),
                                &right,
                            )),
                    )
                })?;
                let type_ = left.type_().clone();
                binary(left, operator, right, type_)
            }

            // (a -> a) -> bool
            BinaryOp::Eq | BinaryOp::Neq => {
                let left = self.infer(left)?;
                let right = self.infer(right)?;
                unify(left.type_().clone(), right.type_().clone()).map_err(|d| {
                    Error::with_diagnostic(
                        d.with_label(primary(left.merge(&right)))
                            .with_label(label(
                                format!("left side has type {}", left.type_()),
                                &left,
                            ))
                            .with_label(label(
                                format!("right side has type {}", right.type_()),
                                &right,
                            )),
                    )
                })?;
                binary(left, operator, right, bool())
            }

            // (number -> number) -> bool
            BinaryOp::Low | BinaryOp::Leq | BinaryOp::Gre | BinaryOp::Geq => {
                let left = self.infer(left)?;
                let right = self.infer(right)?;
                if !left.type_().is_number() || !right.type_().is_number() {
                    return error(
                        diagnostic("left and right side must be numbers", left.merge(&right))
                            .with_label(label(
                                format!("left side has type {}", left.type_()),
                                &left,
                            ))
                            .with_label(label(
                                format!("right side has type {}", right.type_()),
                                &right,
                            )),
                    );
                }
                unify(left.type_().clone(), right.type_().clone()).map_err(|d| {
                    Error::with_diagnostic(
                        d.with_label(primary(left.merge(&right)))
                            .with_label(label(
                                format!("left side has type {}", left.type_()),
                                &left,
                            ))
                            .with_label(label(
                                format!("right side has type {}", right.type_()),
                                &right,
                            )),
                    )
                })?;
                binary(left, operator, right, bool())
            }

            // (bool -> bool) -> bool
            BinaryOp::And | BinaryOp::Or => {
                let left = self.infer(left)?;
                let right = self.infer(right)?;
                unify(left.type_().clone(), bool()).map_err(|d| {
                    Error::with_diagnostic(
                        d.with_label(primary(left.merge(&right))).with_label(label(
                            format!("left side has type {}", left.type_()),
                            &left,
                        )),
                    )
                })?;
                unify(right.type_().clone(), bool()).map_err(|d| {
                    Error::with_diagnostic(d.with_label(primary(left.merge(&right))).with_label(
                        label(format!("right side has type {}", right.type_()), &right),
                    ))
                })?;
                binary(left, operator, right, bool())
            }
        })
    }

    fn fold_get(
        &mut self,
        left: Expr<Self::Type>,
        right: Identifier,
        _: Self::Type,
    ) -> Self::Output {
        let left = self.fold_expr(left)?;
        let object = collapse_links(left.type_().clone());
        let name = match object.as_ref() {
            Type::Named { name, .. } => name,
            _ => {
                return error(
                    diagnostic(
                        "associatens can only be used on named types",
                        left.merge(&right),
                    )
                    .with_label(label(format!("object has type {}", left.type_()), &left)),
                );
            }
        };
        let struct_ = match self.environment.find_struct(name) {
            Some(struct_) => struct_,
            _ => {
                return error(
                    diagnostic("name is not a struct", left.merge(&right))
                        .with_label(label(format!("object has type {}", left.type_()), &left)),
                );
            }
        };
        let type_ = struct_.find_associaten(&right)?.type_().clone();
        Ok(access_struct(left, right, type_))
    }

    fn fold_index(
        &mut self,
        object: Expr<Self::Type>,
        index: Expr<Self::Type>,
        source: Source,
        _: Self::Type,
    ) -> Self::Output {
        let object = self.infer(object)?;
        let index = self.infer(index)?;
        if !index.type_().is_int() {
            return error(diagnostic("index is not an integer", source));
        }
        match collapse_links(object.type_().clone()).as_ref() {
            Type::Array { type_, .. } => Ok(index_array(object, index, source, type_.clone())),
            _ => error(diagnostic("not an array", source)),
        }
    }

    fn fold_call(
        &mut self,
        callee: Expr<Self::Type>,
        arguments: Vec<Expr<Self::Type>>,
        source: Source,
        _: Self::Type,
    ) -> Self::Output {
        let callee = self.infer(callee)?;
        if let Type::Fn {
            arguments: expected,
            return_,
        } = callee.type_().clone().as_ref()
        {
            if arguments.len() != expected.len() {
                return error(diagnostic("argument len differs", &callee));
            }

            let mut args = Vec::with_capacity(arguments.len());
            for (t1, t2) in arguments.into_iter().zip(expected) {
                let arg = self.infer_and_unify(t1, t2.clone())?;
                args.push(arg);
            }
            Ok(call(callee, args, source, return_.clone()))
        } else {
            error(diagnostic("callee is not a function", &callee))
        }
    }

    fn fold_create_array(
        &mut self,
        fields: Vec<Expr<Self::Type>>,
        source: Source,
        type_: Typed,
    ) -> Self::Output {
        if let Type::Array {
            type_: inner_type,
            size,
        } = type_.as_ref()
        {
            let fields = {
                let mut typed_fields = Vec::with_capacity(fields.len());
                for field in fields {
                    let field = self.infer_and_unify(field, inner_type.clone())?;
                    typed_fields.push(field)
                }
                typed_fields
            };
            if *size < fields.len() {
                return error(
                    diagnostic(format!("number of elements exceeds size"), source).with_label(
                        label(
                            format!("expected at most {} elements, got {}", size, fields.len()),
                            source,
                        ),
                    ),
                );
            }
            Ok(TypedExpr::new(ExprKind::CreateArray {
                fields,
                source,
                type_,
            }))
        } else {
            error(diagnostic("not an array type", source))
        }
    }

    fn fold_create_struct(
        &mut self,
        fields: Vec<(Identifier, Expr<Self::Type>)>,
        source: Source,
        type_: Typed,
    ) -> Self::Output {
        let mut folded = Vec::with_capacity(fields.len());
        for (name, expr) in fields {
            // TODO: check expected type
            let expr = self.infer(expr)?;
            folded.push((name, expr));
        }
        Ok(create_struct(folded, source, type_))
    }

    fn fold_if(
        &mut self,
        condition: Expr<Self::Type>,
        if_branch: Box<Stmt<Self::Type>>,
        else_branch: Option<Box<Stmt<Self::Type>>>,
        source: Source,
        _: Self::Type,
    ) -> Self::Output {
        // Check the condition
        let condition = self.fold_expr(condition)?;

        // Fold both branches
        let if_branch = self.fold_stmt(*if_branch)?;
        let else_branch = match else_branch {
            Some(else_branch) => self.fold_stmt(*else_branch)?,
            None => return Ok(if_(condition, if_branch, None, source, void())),
        };

        // Check if the return type of both branches match
        let if_type = match if_branch.kind() {
            StmtKind::ExprStmt(expr) => expr.type_().clone(),
            _ => void(),
        };
        let else_type = match else_branch.kind() {
            StmtKind::ExprStmt(expr) => expr.type_().clone(),
            _ => void(),
        };
        unify(else_type.clone(), if_type.clone()).map_err(|d| {
            Error::with_diagnostic(
                d.with_label(primary(source))
                    .with_label(label(format!("if branch has type {}", if_type), &if_branch))
                    .with_label(label(
                        format!("else branch has type {}", else_type),
                        &else_branch,
                    )),
            )
        })?;
        Ok(if_(
            condition,
            if_branch,
            Some(else_branch),
            source,
            if_type,
        ))
    }

    fn fold_literal(&mut self, value: ExprOrigin<Literal>, _: Self::Type) -> Self::Output {
        let type_ = match value.kind() {
            Literal::Int(_) => int(),
            Literal::Float(_) => float(),
            Literal::Bool(_) => bool(),
            Literal::String(_) => str(),
        };
        Ok(literal(value, type_))
    }

    fn fold_variable(&mut self, name: Identifier, _: Self::Type) -> Self::Output {
        let type_ = self.environment.find_var(&name)?.type_().clone();
        Ok(variable(name, type_))
    }
}
