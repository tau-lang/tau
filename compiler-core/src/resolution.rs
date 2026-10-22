/*
use crate::{
    ast::{
        Location, Typed,
        declaration::{Decl, DeclFunc, DeclStruct, DeclVisitor},
        expression::{Expr, UntypedExprVisitor},
        identifier::Identifier,
        statement::{Stmt, StmtVisitor},
    },
    error::{Result, diagnostic, error, label},
    lexer::{Source, Token, TokenType},
    typing::{TypeCell, TypeDef, TypeNames, TypeTree},
};
use std::{boxed::Box, cell::RefCell, collections::HashMap};

pub struct Resolution<'a> {
    types: &'a TypeTree,
    // Current scope is the list of the scope we are currently in and all scopes above it.
    scopes: Vec<Box<RefCell<TypeNames>>>,
    return_type: Option<Box<TypeDef>>,
}

impl<'a> Resolution<'a> {
    pub fn new(types: &'a TypeTree, fields: TypeNames) -> Resolution<'a> {
        Resolution {
            types,
            scopes: vec![Box::new(RefCell::new(fields))],
            return_type: None,
        }
    }

    pub fn resolve(mut self, declarations: &'a Vec<Decl<()>>) -> Result<TypeNames> {
        for decl in declarations {
            self.visit_decl(decl)?;
        }
        Ok(self
            .scopes
            .first()
            .expect("expect number of closed scopes is not higher then open scopes")
            .take())
    }

    fn declare_variable(&mut self, var_name: &Identifier, var_type: Box<TypeDef>) -> Result<()> {
        if (*self
            .scopes
            .last_mut()
            .expect("expected there to be at least one scope"))
        .borrow_mut()
        .insert(var_name.to_string(), var_type)
        .is_some()
        {
            return error(diagnostic(
                format!("variable '{}' was already declared", var_name.name()),
                var_name.source(),
            ));
        }
        Ok(())
    }

    fn lookup_variable(&mut self, var_name: &Identifier) -> Result<Box<TypeDef>> {
        if let Some(scope) = self.scopes.last() {
            if let Some(v) = scope.borrow().get(var_name.name()) {
                Ok(v.clone())
            } else {
                let origin = self.scopes.pop().expect("expected scope");
                let var_type = self.lookup_variable(var_name);
                self.scopes.push(origin);
                var_type
            }
        } else {
            error(diagnostic(
                format!("could not find variable '{}'", var_name.name()),
                var_name.source(),
            ))
        }
    }

    fn member(&self, lookup: Box<TypeDef>, member_name: &'a Identifier) -> Result<Box<TypeDef>> {
        if let TypeDef::Struct(structure) = lookup.as_ref() {
            if let Some(ref_type) = structure.fields.get(member_name.name()) {
                self.types.lookup_type(&ref_type.clone())
            } else {
                error(diagnostic(
                    format!("expected struct {lookup} contains field {member_name}"),
                    member_name.source(),
                ))
            }
        } else {
            error(diagnostic(
                format!("expected struct, got {lookup} which has no {member_name}"),
                member_name.source(),
            ))
        }
    }

    fn begin_scope(&mut self) {
        let ref_counter = Box::new(RefCell::new(HashMap::new()));
        self.scopes.push(ref_counter);
    }

    fn end_scope(&mut self) {
        self.scopes.pop().expect("expect end scope");
    }
}

impl<'a> UntypedExprVisitor<'a> for Resolution<'a> {
    type Type = ();
    type Output = Result<Box<TypeDef>>;

    fn visit_unary(&mut self, operator: &Token, right: &'a Box<Expr<Self::Type>>) -> Self::Output {
        let r = self.visit_expr(right)?;
        let throw_error = |msg: &str| {
            return error(
                diagnostic(
                    msg.to_string(),
                    &Source::merge(&operator.source(), &right.source()),
                )
                .with_label(label(
                    format!("right hand side has type {}", r),
                    right.source(),
                )),
            );
        };
        match operator.token_type() {
            TokenType::Not => {
                if !r.is_bool() {
                    return throw_error("the value is not a boolean and cannot be negated");
                }
                Ok(r)
            }
            TokenType::Add | TokenType::Sub => {
                if !r.is_number() {
                    return throw_error(
                        "the type is not a number and thus cannot be used like one (+/-)",
                    );
                }
                Ok(r)
            }
            TokenType::Star => {
                if let TypeDef::RawPointer(inner) = r.as_ref() {
                    Ok(inner.clone())
                } else {
                    return throw_error("can only deref pointers");
                }
            }
            TokenType::Ref => Ok(Box::new(TypeDef::RawPointer(r))),
            _ => unreachable!("{:?}", operator),
        }
    }

    fn visit_binary(
        &mut self,
        left: &'a Box<Expr<Self::Type>>,
        operator: &Token,
        right: &'a Box<Expr<Self::Type>>,
    ) -> Result<Box<TypeDef>> {
        let l = self.visit_expr(left)?;
        let r = self.visit_expr(right)?;
        let throw_error = |msg: &str| {
            return error(
                diagnostic(
                    msg.to_string(),
                    &Source::merge(&left.source(), &right.source()),
                )
                .with_label(label(
                    format!("left hand side has type {}", l),
                    left.source(),
                ))
                .with_label(label(
                    format!("right hand side has type {}", r),
                    right.source(),
                )),
            );
        };
        match operator.token_type() {
            TokenType::Add
            | TokenType::Sub
            | TokenType::Star
            | TokenType::Div
            | TokenType::SetAdd
            | TokenType::SetSub
            | TokenType::SetStar
            | TokenType::SetDiv => {
                if !l.is_number() {
                    throw_error("the left-hand-side is expected to be a number")?;
                }
                if !r.is_number() {
                    throw_error("the right-hand-side is expected to be a number")?;
                }
                if l.is_castable_to(&r) {
                    return Ok(r);
                } else if r.is_castable_to(&l) {
                    return Ok(l);
                }
                throw_error("number types of left and right side do not match")
            }
            TokenType::And | TokenType::Or | TokenType::Vbar => {
                if !l.is_bool() {
                    throw_error("the left-hand-side is expected to be a boolean")?;
                }
                if !r.is_bool() {
                    throw_error("the right-hand-side is expected to be a boolean")?;
                }
                Ok(l)
            }
            TokenType::Leq | TokenType::Low | TokenType::Geq | TokenType::Gre => {
                if !l.is_number() {
                    return throw_error("the left-hand-side is expected to be a number");
                }
                if !r.is_number() {
                    return throw_error("the right-hand-side is expected to be a number");
                }
                Ok(self.types.lookup_name("bool"))
            }
            TokenType::Eq | TokenType::Neq => match (l.is_number(), r.is_number()) {
                (true, true) => Ok(self.types.lookup_name("bool")),
                (true, false) => throw_error(
                    "since the left-hand-side is a number, the right-hand-side is expected to be a number too",
                ),
                (false, true) => throw_error(
                    "since the right-hand-side is a number, the left-hand-side is expected to be a number too",
                ),
                (false, false) => {
                    if l == r {
                        Ok(self.types.lookup_name("bool"))
                    } else {
                        throw_error("the types are expected to be the same")
                    }
                }
            },
            TokenType::Set => {
                if !r.is_castable_to(&l) {
                    throw_error("can not cast right side of set expression to expected var type")?;
                }
                Ok(l)
            }
            _ => unreachable!("{:?}", operator),
        }
    }

    fn visit_get(
        &mut self,
        left: &'a Box<Expr<Self::Type>>,
        right: &'a Identifier,
        lookup: &'a TypeCell,
    ) -> Result<Box<TypeDef>> {
        let l = self.visit_expr(left)?;
        *lookup.borrow_mut() = l.clone();
        self.member(l, right)
    }

    fn visit_index(
        &mut self,
        object: &'a Box<Expr<Self::Type>>,
        index: &'a Box<Expr<Self::Type>>,
        lookup: &'a TypeCell,
    ) -> Result<Box<TypeDef>> {
        let l = self.visit_expr(object)?;
        let r = self.visit_expr(index)?;
        if let TypeDef::Array(array_type) = l.as_ref() {
            if !r.is_integer() {
                return error(
                    diagnostic(
                        "index has to be an integer".to_string(),
                        &Source::merge(object.source(), index.source()),
                    )
                    .with_label(label(format!("object has type {}", l), object.source()))
                    .with_label(label(format!("index has type {}", r), index.source())),
                );
            }
            let ref_type = self.types.lookup_type(&array_type)?;
            *lookup.borrow_mut() = ref_type.clone();
            Ok(ref_type)
        } else {
            error(
                diagnostic(
                    "expected to index into an array".to_string(),
                    object.source(),
                )
                .with_label(label(format!("object has type {}", l), object.source()))
                .with_label(label(format!("index has type {}", r), index.source())),
            )
        }
    }

    fn visit_call(
        &mut self,
        callee: &'a Box<Expr<Self::Type>>,
        arguments: &'a [Box<Expr<Self::Type>>],
    ) -> Result<Box<TypeDef>> {
        let callee_type = self.visit_expr(callee)?;
        if let TypeDef::Function(function) = &*callee_type {
            if arguments.len() != function.parameters.len() {
                return error(
                    diagnostic(
                        format!(
                            "expected {} arguments, got {}",
                            function.parameters.len(),
                            arguments.len()
                        ),
                        callee.source(),
                    )
                    .with_label(label("function was defined here", callee.source())),
                );
            }

            for (argument, para_type) in arguments.iter().zip(&function.parameters) {
                let arg_type = self.visit_expr(argument)?;
                if !arg_type.is_castable_to(&*self.types.lookup_type(para_type)?) {
                    return error(diagnostic(
                        format!(
                            "argument type '{}' supplied to function does not match parameter type '{}'",
                            arg_type, para_type
                        ),
                        callee.source(),
                    ));
                }
            }
            self.types.lookup_type(&function.return_type)
        } else {
            return error(diagnostic(
                "expected call a function".to_string(),
                callee.source(),
            ));
        }
    }

    fn visit_create_array(
        &mut self,
        array_type: &'a TypeCell,
        array_size: &'a Option<Box<Expr<Self::Type>>>,
        fields: &'a [Box<Expr<Self::Type>>],
    ) -> Result<Box<TypeDef>> {
        if let Some(size_expr) = array_size {
            if !self.visit_expr(size_expr)?.is_integer() {
                return error(diagnostic(
                    "expected size of array to be an integer".to_string(),
                    size_expr.source(),
                ));
            }
        }
        let ref_type = self.types.lookup_type(&array_type.borrow())?;
        *array_type.borrow_mut() = ref_type.clone();

        for field in fields {
            if !self.visit_expr(field)?.is_castable_to(&ref_type) {
                return error(diagnostic(
                    "expected size of array to be an integer".to_string(),
                    field.source(),
                ));
            }
        }

        Ok(Box::new(TypeDef::Array(ref_type)))
    }

    fn visit_create_struct(
        &mut self,
        struct_type: &'a TypeCell,
        fields: &'a [(Identifier, Box<Expr<Self::Type>>)],
    ) -> Result<Box<TypeDef>> {
        let ref_type = self.types.lookup_type(&struct_type.borrow().clone())?;

        for (field_name, field_expr) in fields {
            let field_type = self.visit_expr(field_expr)?;
            let expected_type = &*self.member(ref_type.clone(), field_name)?;
            if !field_type.is_castable_to(expected_type) {
                return error(diagnostic(
                    format!(
                        "could not cast parameter {field_name} of type {field_type} to {expected_type}"
                    ),
                    field_expr.source(),
                ));
            }
        }
        *struct_type.borrow_mut() = ref_type.clone();
        Ok(ref_type)
    }

    fn visit_if(
        &mut self,
        condition: &'a Box<Expr<Self::Type>>,
        if_branch: &'a Box<Stmt<Self::Type>>,
        else_branch: &'a Option<Box<Stmt<Self::Type>>>,
        expression_type: &'a TypeCell,
    ) -> Result<Box<TypeDef>> {
        // Check condition
        let c = self.visit_expr(condition)?;
        if !c.is_bool() {
            return error(
                diagnostic(
                    "if condition must be a boolean",
                    &Source::merge(condition.source(), if_branch.source()),
                )
                .with_label(label(
                    format!("condition has type {}", c),
                    condition.source(),
                )),
            );
        }

        // Check if type
        let if_type = match self.visit_stmt(if_branch)? {
            Some(ty) => self.types.lookup_type(&ty)?,
            None => return Ok(self.types.lookup_name("void")),
        };

        let Some(else_branch) = else_branch else {
            let void = self.types.lookup_name("void");

            if if_type == void {
                return Ok(void);
            }

            return error(diagnostic(
                "if expression expected else branch".to_string(),
                &Source::merge(&condition.source(), &if_branch.source()),
            ));
        };

        // Check else type
        let else_type = match self.visit_stmt(else_branch)? {
            Some(ty) => self.types.lookup_type(&ty)?,
            None => self.types.lookup_name("void"),
        };

        // Check if both branches have the same type
        let result_type = if if_type.is_castable_to(&else_type) {
            else_type
        } else if else_type.is_castable_to(&if_type) {
            if_type
        } else {
            return error(
                diagnostic(
                    format!("if expression has not the same type for if and else"),
                    &Source::merge(&condition.source(), &else_branch.source()),
                )
                .with_label(label(
                    format!("if expression has type {}", if_type),
                    if_branch.source(),
                ))
                .with_label(label(
                    format!("else expression has type {}", else_type),
                    else_branch.source(),
                )),
            );
        };

        *expression_type.borrow_mut() = result_type.clone();
        Ok(result_type)
    }

    fn visit_literal(&mut self, value: &Token) -> Result<Box<TypeDef>> {
        Ok(self.types.lookup_name(match value.token_type() {
            TokenType::Number(_) => "i32",
            TokenType::String(_) => "str",
            TokenType::Bool(_) => "bool",
            _ => unreachable!(),
        }))
    }

    fn visit_variable(
        &mut self,
        name: &Identifier,
        variable_type: &'a TypeCell,
    ) -> Result<Box<TypeDef>> {
        let real_type = self.lookup_variable(name)?;
        *variable_type.borrow_mut() = real_type.clone();
        Ok(real_type)
    }
}

impl<'a> StmtVisitor<'a> for Resolution<'a> {
    type Type = ();
    type Output = Result<Option<Box<TypeDef>>>;

    fn visit_block(&mut self, statements: &'a [Box<Stmt<Self::Type>>]) -> Self::Output {
        let mut errors = Option::None;
        self.begin_scope();
        for stmt in statements {
            if self.return_type.is_some() {
                return error(diagnostic(
                    "function has already returned".to_string(),
                    stmt.source(),
                ));
            }
            if let Err(err) = self.visit_stmt(stmt) {
                errors = err.after(errors);
            }
        }
        self.end_scope();
        if let Some(err) = errors {
            return Err(err);
        }
        Ok(None)
    }

    fn visit_let(
        &mut self,
        name: &'a Identifier,
        var_type: &'a TypeCell,
        initializer: &'a Expr<Self::Type>,
    ) -> Self::Output {
        let real_type = self.visit_expr(initializer)?;
        let expected_type = self.types.lookup_type(&*var_type.borrow())?;
        if let TypeDef::Unknown = *expected_type {
            self.declare_variable(name, real_type.clone())?;
            *var_type.borrow_mut() = real_type;
        } else if real_type.is_castable_to(&expected_type) {
            self.declare_variable(name, expected_type.clone())?;
            *var_type.borrow_mut() = expected_type;
        } else {
            return error(diagnostic(
                format!("expr type {real_type} can not be cast to {expected_type}"),
                initializer.source(),
            ));
        }
        Ok(None)
    }

    fn visit_return(&mut self, value: &'a Expr<Self::Type>) -> Self::Output {
        self.return_type = Some(self.visit_expr(value)?);
        Ok(None)
    }

    fn visit_break(&mut self) -> Self::Output {
        Ok(None)
    }

    fn visit_while(&mut self, condition: &'a Expr<Self::Type>, body: &'a Box<Stmt<Self::Type>>) -> Self::Output {
        self.begin_scope();
        if !self.visit_expr(condition)?.is_bool() {
            return error(diagnostic(
                "expected while confition to be boolean".to_string(),
                condition.source(),
            ));
        }
        self.visit_stmt(body)?;
        self.end_scope();
        Ok(None)
    }

    fn visit_expr_stmt(&mut self, expr: &'a Expr<Self::Type>) -> Self::Output {
        Ok(Some(self.visit_expr(expr)?))
    }
}

impl<'a> DeclVisitor<'a> for Resolution<'a> {
    type Type = ();
    type Output = Result<()>;

    fn visit_import(&mut self, _: &[Identifier], _: &Source) -> Self::Output {
        Ok(())
    }

    fn visit_struct(&mut self, structure: &'a DeclStruct, source: &'a Source) -> Self::Output {
        self.begin_scope();
        let struct_type = self
            .types
            .lookup_path(&vec![
                // TODO: add full module path
                structure.name.clone(),
            ])
            .expect("expected to find own struct name when trying to reference self")
            .clone();
        (*self.scopes.last_mut().expect("expected scope"))
            .borrow_mut()
            .insert("self".to_string(), struct_type);
        for (_, member_type) in &structure.fields {
            let ref_type = self.types.lookup_type(&member_type.borrow().clone())?;
            *member_type.borrow_mut() = ref_type;
        }
        self.end_scope();
        Ok(())
    }

    fn visit_function(&mut self, function: &'a DeclFunc<Self::Type>, source: &'a Source) -> Self::Output {
        if self.return_type.is_some() {
            return error(diagnostic(
                "return type already exists before body was defined".to_string(),
                function.name.source(),
            ));
        }
        let ref_type = self
            .types
            .lookup_type(&function.return_type.borrow().clone())?;
        *function.return_type.borrow_mut() = ref_type.clone();

        // When a new body begins, first define all parameters as local
        // variables.
        self.begin_scope();
        for (param_name, param_type) in &function.params {
            let ref_type = self.types.lookup_type(&param_type.borrow().clone())?;
            self.declare_variable(&param_name, ref_type.clone())?;
            *param_type.borrow_mut() = ref_type;
        }
        if function.modifiers.is_extern {
            // This function is defined outside of tau, there is no body that we
            // can typecheck.
            self.end_scope();
            return Ok(());
        }

        // Check body. If an error occures, collect errors instead of throwing
        // them directly.
        let mut errors = Option::None;
        for stmt in &function.body {
            if let Err(err) = self.visit_stmt(stmt) {
                errors = err.after(errors);
            }
        }
        if let Some(err) = errors {
            return Err(err);
        }

        // Calculate what was declared as a return type and what we really got.
        let real = self.types.lookup_type(
            &self
                .return_type
                .clone()
                .unwrap_or_else(|| self.types.lookup_name("void")),
        )?;
        if !real.is_castable_to(ref_type.as_ref()) {
            return error(diagnostic(
                format!("could not cast return type {} to {}", real, ref_type),
                function.name.source(),
            ));
        }
        self.return_type = None;
        self.end_scope();
        Ok(())
    }

    fn visit_const(
        &mut self,
        var_name: &'a Identifier,
        initializer: &'a Expr<Self::Type>,
        source: &'a Source,
    ) -> Self::Output {
        let expected = self.types.lookup_type(&var_type.borrow().clone())?;
        let actual = self.visit_expr(initializer)?;
        if !actual.is_castable_to(&expected) {
            return error(diagnostic(
                format!(
                    "const '{}' has the type {}, but was declared as {}",
                    var_name, actual, expected
                ),
                &Source::merge(&var_name.source(), &initializer.source()),
            ));
        }
        Ok(())
    }
}
*/
