use crate::{
    analyzer::SemanticAnalyzer,
    ast::{
        Identifier, Location, TypedExpr, Unfold,
        expression::{Expr, ExprFolder, ExprKind, TypedExprVisitor},
        fold_vec,
        lookup::{AdlLookup, CallLookup, VarLookup},
        origin::{BinaryOp, Literal, Origin, UnaryOp},
        prelude::*,
    },
    error::{Error, Result, diagnostic, error, label, primary},
    lexer::Source,
    type_::{Type, Typed, Untyped, collapse_links, prelude::*, unify},
};
use codespan_reporting::diagnostic::Diagnostic;

impl<'a> ExprFolder for SemanticAnalyzer<'a> {
    type Type = Untyped;
    type Output = Result<TypedExpr>;

    fn fold_unary(
        &mut self,
        operator: Origin<UnaryOp>,
        right: Expr<Self::Type>,
        _: Self::Type,
    ) -> Self::Output {
        let right = self.fold_expr(right)?;
        match operator.kind() {
            UnaryOp::Neg => {
                let type_ = right.type_().clone();
                Ok(unary(operator, right, type_))
            }
            UnaryOp::Not => {
                unify(right.type_().clone(), bool())
                    .map_err(|d| Error::new(d.with_label(primary(operator.merge(&right)))))?;
                Ok(unary(operator, right, bool()))
            }
            UnaryOp::Ref => {
                let type_ = ptr(right.type_().clone(), true);
                Ok(unary(operator, right, type_))
            }
            UnaryOp::Deref => {
                // Type must be known here
                let type_ = collapse_links(right.type_().clone());
                match type_.as_ref() {
                    Type::Ptr { type_, .. } => {
                        let inner_type = type_.clone();
                        Ok(unary(operator, right, inner_type))
                    }
                    _ => error(
                        diagnostic(
                            format!("expected pointer, got {}", right.type_()),
                            operator.merge(&right),
                        )
                        .with_label(label(format!("right has type {}", right.type_()), &right)),
                    ),
                }
            }
        }
    }

    fn fold_binary(
        &mut self,
        left: Expr<Self::Type>,
        operator: Origin<BinaryOp>,
        right: Expr<Self::Type>,
        _: Self::Type,
    ) -> Self::Output {
        Ok(match operator.kind() {
            // (a -> a) -> a
            BinaryOp::Set => {
                let left = self.infer(left)?;
                let right = self.infer(right)?;
                unify(left.type_().clone(), right.type_().clone()).map_err(|d| {
                    Error::new(
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
                unify(left.type_().clone(), right.type_().clone()).map_err(|d| {
                    Error::new(
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
                    Error::new(
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
                unify(left.type_().clone(), right.type_().clone()).map_err(|d| {
                    Error::new(
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
                    Error::new(
                        d.with_label(primary(left.merge(&right))).with_label(label(
                            format!("left side has type {}", left.type_()),
                            &left,
                        )),
                    )
                })?;
                unify(right.type_().clone(), bool()).map_err(|d| {
                    Error::new(d.with_label(primary(left.merge(&right))).with_label(label(
                        format!("right side has type {}", right.type_()),
                        &right,
                    )))
                })?;
                binary(left, operator, right, bool())
            }
        })
    }

    fn fold_access_struct(
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
        let type_ = struct_.find_associated(&right)?.kind().clone();
        Ok(access_struct(left, right, type_))
    }

    fn fold_index_array(
        &mut self,
        object: Expr<Self::Type>,
        index: Expr<Self::Type>,
        source: Source,
        _: Self::Type,
    ) -> Self::Output {
        let object = self.infer(object)?;
        let index = self.infer_and_unify(index, usize())?;
        match collapse_links(object.type_().clone()).as_ref() {
            Type::Array { type_, .. } => Ok(index_array(object, index, source, type_.clone())),
            _ => error(diagnostic("not an array", source)),
        }
    }

    fn fold_call(
        &mut self,
        lookup: CallLookup<Self::Type>,
        arguments: Vec<Expr<Self::Type>>,
        source: Source,
        _: Self::Type,
    ) -> Self::Output {
        let mut arguments = fold_vec(arguments, |expr| self.infer(expr))?;
        let (name, kind) = lookup.unfold();
        let (lookup, type_def) = match kind {
            AdlLookup::Lookup(lookup) => {
                let type_def = self.environment.find_symbol(&lookup, &name)?;
                let lookup = CallLookup::unqualified(name);
                (lookup, type_def)
            }
            AdlLookup::Adl(callee) => {
                let callee = self.infer(*callee)?;
                let _type_ = callee.type_().clone();
                arguments.insert(0, callee);
                // TODO: Find fully qualified path from type.
                todo!()
            }
        };
        if let Type::Fn {
            arguments: expected,
            return_,
        } = type_def.kind().as_ref()
        {
            if arguments.len() != expected.len() {
                return error(diagnostic("argument len differs", &lookup));
            }
            for (i, (t1, t2)) in arguments.iter().zip(expected).enumerate() {
                unify(t1.type_().clone(), t2.clone()).map_err(|d| {
                    Error::new(
                        d.with_label(label(
                            format!("parameter {} expected type {}", i, t2),
                            &lookup,
                        ))
                        .with_label(label(format!("but got {} instead", t1.type_()), t1)),
                    )
                })?;
            }
            Ok(call(lookup, arguments, source, return_.clone()))
        } else {
            error(diagnostic("callee is not a function", &lookup))
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
                    diagnostic("number of elements exceeds size", source).with_label(label(
                        format!("expected at most {} elements, got {}", size, fields.len()),
                        source,
                    )),
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

    fn fold_ternary(
        &mut self,
        condition: Expr<Self::Type>,
        if_branch: Expr<Self::Type>,
        else_branch: Expr<Self::Type>,
        source: Source,
        _: Self::Type,
    ) -> Self::Output {
        // Check the condition
        let condition = self.fold_expr(condition)?;

        // Fold both branches
        let if_branch = self.fold_expr(if_branch)?;
        let else_branch = self.fold_expr(else_branch)?;

        // Check if the return type of both branches match
        let if_type = if_branch.type_();
        let else_type = else_branch.type_();
        unify(else_type.clone(), if_type.clone()).map_err(|d| {
            Error::new(
                d.with_label(primary(source))
                    .with_label(label(format!("if branch has type {}", if_type), &if_branch))
                    .with_label(label(
                        format!("else branch has type {}", else_type),
                        &else_branch,
                    )),
            )
        })?;
        let type_ = if_type.clone();
        Ok(ternary(condition, if_branch, else_branch, source, type_))
    }

    fn fold_literal(&mut self, value: Origin<Literal>, _: Self::Type) -> Self::Output {
        let type_ = match value.kind() {
            Literal::Int(_) => self.new_unbound_var(),
            Literal::Float(_) => self.new_unbound_var(),
            Literal::Bool(_) => bool(),
            Literal::String(_) => str(),
        };
        Ok(literal(value, type_))
    }

    fn fold_variable(&mut self, lookup: VarLookup, _: Self::Type) -> Self::Output {
        let origin = self.environment.find_var(lookup.kind(), lookup.name())?;
        Ok(variable(lookup, link(origin.kind().clone())))
    }
}

impl<'a, 'b> TypedExprVisitor<'a> for SemanticAnalyzer<'b> {
    type Type = Typed;
    type Output = Result<()>;

    fn visit_unary(
        &mut self,
        operator: &'a Origin<UnaryOp>,
        right: &'a Expr<Self::Type>,
        _: &'a Self::Type,
    ) -> Self::Output {
        self.visit_expr(right)?;
        match operator.kind() {
            UnaryOp::Neg => {
                if !right.type_().is_number() {
                    return error(
                        diagnostic(
                            format!("expected number, got {}", right.type_()),
                            operator.merge(right),
                        )
                        .with_label(label(format!("right has type {}", right.type_()), right)),
                    );
                }
            }
            UnaryOp::Not => {}
            UnaryOp::Ref => {}
            UnaryOp::Deref => {}
        }
        Ok(())
    }

    fn visit_binary(
        &mut self,
        left: &'a Expr<Self::Type>,
        operator: &'a Origin<BinaryOp>,
        right: &'a Expr<Self::Type>,
        _: &'a Self::Type,
    ) -> Self::Output {
        self.visit_expr(left)?;
        self.visit_expr(right)?;

        Ok(match operator.kind() {
            // (a -> a) -> a
            BinaryOp::Set => {}

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
                if !left.type_().is_number() || !right.type_().is_number() {
                    return error(
                        diagnostic("left and right side must be numbers", left.merge(right))
                            .with_label(label(format!("left side has type {}", left.type_()), left))
                            .with_label(label(
                                format!("right side has type {}", right.type_()),
                                right,
                            )),
                    );
                }
            }

            // (a -> a) -> bool
            BinaryOp::Eq | BinaryOp::Neq => {}

            // (number -> number) -> bool
            BinaryOp::Low | BinaryOp::Leq | BinaryOp::Gre | BinaryOp::Geq => {
                if !left.type_().is_number() || !right.type_().is_number() {
                    return error(
                        diagnostic("left and right side must be numbers", left.merge(right))
                            .with_label(label(format!("left side has type {}", left.type_()), left))
                            .with_label(label(
                                format!("right side has type {}", right.type_()),
                                right,
                            )),
                    );
                }
            }

            // (bool -> bool) -> bool
            BinaryOp::And | BinaryOp::Or => {}
        })
    }

    fn visit_access_struct(
        &mut self,
        left: &'a Expr<Self::Type>,
        _: &'a Identifier,
        _: &'a Self::Type,
    ) -> Self::Output {
        self.visit_expr(left)
    }

    fn visit_index_array(
        &mut self,
        object: &'a Expr<Self::Type>,
        index: &'a Expr<Self::Type>,
        source: &'a Source,
        _: &'a Self::Type,
    ) -> Self::Output {
        self.visit_expr(object)?;
        if !index.type_().is_int() {
            return error(
                diagnostic("index is not an integer", *source)
                    .with_label(label(format!("index has type {}", index.type_()), index)),
            );
        }
        self.visit_expr(index)
    }

    fn visit_call(
        &mut self,
        _: &'a CallLookup<Self::Type>,
        arguments: &'a [Expr<Self::Type>],
        _: &'a Source,
        _: &'a Self::Type,
    ) -> Self::Output {
        for argument in arguments {
            self.visit_expr(argument)?;
        }
        Ok(())
    }

    fn visit_create_array(
        &mut self,
        fields: &'a [Expr<Self::Type>],
        _: &'a Source,
        _: &'a Typed,
    ) -> Self::Output {
        for field in fields {
            self.visit_expr(field)?;
        }
        Ok(())
    }

    fn visit_create_struct(
        &mut self,
        fields: &'a [(Identifier, Expr<Self::Type>)],
        _: &'a Source,
        _: &'a Typed,
    ) -> Self::Output {
        for (_, init) in fields {
            self.visit_expr(init)?;
        }
        Ok(())
    }

    fn visit_ternary(
        &mut self,
        condition: &'a Expr<Self::Type>,
        if_branch: &'a Expr<Self::Type>,
        else_branch: &'a Expr<Self::Type>,
        _: &'a Source,
        _: &'a Self::Type,
    ) -> Self::Output {
        self.visit_expr(condition)?;
        self.visit_expr(if_branch)?;
        self.visit_expr(else_branch)
    }

    fn visit_literal(&mut self, value: &'a Origin<Literal>, type_: &'a Self::Type) -> Self::Output {
        match value.kind() {
            Literal::Int(_) => {
                if type_.is_unbound() {
                    Err(Error::from(vec![
                        diagnostic(
                            format!("size of AnyInt could not be infered"),
                            value.source(),
                        )
                        .with_label(label(
                            "exect size (e.g. i64) must be known here",
                            value.source(),
                        )),
                        Diagnostic::help()
                            .with_message("bind the expression with let to a congrete type")
                            .with_label(primary(value.source())),
                    ]))
                } else if !type_.is_int() {
                    error(diagnostic(
                        format!("expected {}, got AnyInt", type_),
                        value.source(),
                    ))
                } else {
                    Ok(())
                }
            }
            Literal::Float(_) => {
                if type_.is_unbound() {
                    Err(Error::from(vec![
                        diagnostic(
                            format!("size of AnyFloat could not be infered"),
                            value.source(),
                        )
                        .with_label(label(
                            "exect size (e.g. f64) must be known here",
                            value.source(),
                        )),
                        Diagnostic::help()
                            .with_message("bind the expression with let to a congrete type")
                            .with_label(primary(value.source())),
                    ]))
                } else if !type_.is_float() {
                    error(diagnostic(
                        format!("expected {}, got AnyFloat", type_),
                        value.source(),
                    ))
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    fn visit_variable(&mut self, _: &'a VarLookup, _: &'a Self::Type) -> Self::Output {
        Ok(())
    }
}
