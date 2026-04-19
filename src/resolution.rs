use crate::{
    ast::{
        declaration::{Decl, DeclVisitor},
        expression::{Expr, ExprVisitor},
        identifier::Identifier,
        statement::{Stmt, StmtVisitor},
    },
    error::{Diagnostic, Error, Result},
    lexer::{Source, Token, TokenType},
    typing::{TypeCell, TypeDef, TypeNames},
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Debug)]
pub struct Resolution<'a> {
    types: &'a TypeNames,
    // Current scope is the list of the scope we are currently in and all scopes above it.
    scopes: Vec<Rc<RefCell<TypeNames>>>,
    return_type: Option<Rc<TypeDef>>,
}

impl<'a> Resolution<'a> {
    pub fn new(types: &'a TypeNames, fields: TypeNames) -> Resolution<'a> {
        Resolution {
            types,
            scopes: vec![Rc::new(RefCell::new(fields))],
            return_type: None,
        }
    }

    pub fn resolve(mut self, declarations: &'a Vec<Decl>) -> Result<Self> {
        for decl in declarations {
            self.visit_decl(decl)?;
        }
        // When we create a new resolution, we already start in the global scope.
        // After visiting all declarations, we need to end the global scope and
        // push it to all scopes.
        self.end_scope();
        Ok(self)
    }

    fn declare_variable(&mut self, var_name: &Identifier, var_type: Rc<TypeDef>) -> Result<()> {
        if (*self
            .scopes
            .last_mut()
            .expect("expected there to be at least one scope"))
        .borrow_mut()
        .insert(var_name.to_string(), var_type)
        .is_some()
        {
            return Err(Error::new(vec![Diagnostic::new(
                format!("variable '{}' was already declared", var_name.get_name()),
                var_name.get_source(),
            )]));
        }
        Ok(())
    }

    fn lookup_variable(&mut self, var_name: &Identifier) -> Result<Rc<TypeDef>> {
        if let Some(scope) = self.scopes.last() {
            if let Some(v) = scope.borrow().get(var_name.get_name()) {
                Ok(v.clone())
            } else {
                let origin = self.scopes.pop().expect("expected scope");
                let var_type = self.lookup_variable(var_name);
                self.scopes.push(origin);
                var_type
            }
        } else {
            Err(Error::new(vec![Diagnostic::new(
                format!("could not find variable '{}'", var_name.get_name()),
                var_name.get_source(),
            )]))
        }
    }

    fn get_type(&self, name: &str) -> Result<Rc<TypeDef>> {
        let ref_type: Rc<TypeDef> = self
            .types
            .get(name)
            .unwrap_or_else(|| {
                panic!(
                    "expected type name does not exist '{}' in {:#?}",
                    name, self
                )
            })
            .clone();
        self.get_ref_type(ref_type)
    }

    fn get_ref_type(&self, ref_type: Rc<TypeDef>) -> Result<Rc<TypeDef>> {
        if let TypeDef::Lazy(lazy_name) = ref_type.as_ref() {
            self.get_type(lazy_name)
        } else {
            Ok(ref_type)
        }
    }

    fn get_member(&self, lookup: Rc<TypeDef>, member_name: &'a Identifier) -> Result<Rc<TypeDef>> {
        if let TypeDef::Struct { name: _, members } = lookup.as_ref() {
            let ref_type: Rc<TypeDef> = members
                .get(member_name.get_name())
                .expect("expected struct contains field")
                .clone();
            if let TypeDef::Lazy(lazy_name) = ref_type.as_ref() {
                self.get_type(lazy_name)
            } else {
                Ok(ref_type)
            }
        } else if let TypeDef::Module { types: _, fields } = lookup.as_ref() {
            let ref_type = fields
                .get(member_name.get_name())
                .expect("expected modue contains field")
                .clone();
            if let TypeDef::Lazy(lazy_name) = ref_type.as_ref() {
                self.get_type(lazy_name)
            } else {
                Ok(ref_type)
            }
        } else {
            Err(Error::new(vec![Diagnostic::new(
                "expected struct".to_string(),
                member_name.get_source(),
            )]))
        }
    }

    fn begin_scope(&mut self) {
        let ref_counter = Rc::new(RefCell::new(HashMap::new()));
        self.scopes.push(ref_counter);
    }

    fn end_scope(&mut self) {
        self.scopes.pop().expect("expect end scope");
    }
}

impl<'a> ExprVisitor<'a> for Resolution<'a> {
    type Output = Result<Rc<TypeDef>>;

    fn visit_unary(&mut self, operator: &Token, right: &'a Rc<Expr>) -> Result<Rc<TypeDef>> {
        let throw_error = |msg: &str| {
            Err(Error::new(vec![Diagnostic::new(
                msg.to_string(),
                Source::union(&operator.get_source(), &right.source()),
            )]))
        };
        let r = self.visit_expr(right)?;
        match operator.get_type() {
            TokenType::Not => {
                if !r.is_bool() {
                    throw_error("the value is not a boolean and cannot be negated")?;
                }
                Ok(r)
            }
            TokenType::Add | TokenType::Sub => {
                if !r.is_number() {
                    throw_error("the type is not a number and thus cannot be used like one (+/-)")?;
                }
                Ok(r)
            }
            _ => unreachable!("{:?}", operator),
        }
    }

    fn visit_binary(
        &mut self,
        left: &'a Rc<Expr>,
        operator: &Token,
        right: &'a Rc<Expr>,
    ) -> Result<Rc<TypeDef>> {
        let throw_error = |msg: &str| {
            Err(Error::new(vec![Diagnostic::new(
                msg.to_string(),
                Source::union(&left.source(), &right.source()),
            )]))
        };
        let l = self.visit_expr(left)?;
        let r = self.visit_expr(right)?;
        match operator.get_type() {
            TokenType::Add
            | TokenType::Sub
            | TokenType::Mul
            | TokenType::Div
            | TokenType::SetAdd
            | TokenType::SetSub
            | TokenType::SetMul
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
            TokenType::And | TokenType::Or | TokenType::Xor => {
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
                    throw_error("the left-hand-side is expected to be a number")?;
                }
                if !r.is_number() {
                    throw_error("the right-hand-side is expected to be a number")?;
                }
                self.get_type("bool")
            }
            TokenType::Eq | TokenType::Neq => match (l.is_number(), r.is_number()) {
                (true, true) => Ok(l),
                (true, false) => throw_error(
                    "since the left-hand-side is a number, the right-hand-side is expected to be a number too",
                ),
                (false, true) => throw_error(
                    "since the right-hand-side is a number, the left-hand-side is expected to be a number too",
                ),
                (false, false) => {
                    if l == r {
                        Ok(l)
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
        left: &'a Rc<Expr>,
        right: &'a Identifier,
        lookup: &'a TypeCell,
    ) -> Result<Rc<TypeDef>> {
        let l = self.visit_expr(left)?;
        *lookup.borrow_mut() = l.clone();
        self.get_member(l, right)
    }

    fn visit_index(
        &mut self,
        object: &'a Rc<Expr>,
        index: &'a Rc<Expr>,
        looup: &'a TypeCell,
    ) -> Result<Rc<TypeDef>> {
        let l = self.visit_expr(object)?;
        if let TypeDef::Array(array_type) = &*l {
            let r = self.visit_expr(index)?;
            if !r.is_integer() {
                Err(Error::new(vec![Diagnostic::new(
                    "index has to be an integer".to_string(),
                    index.source(),
                )]))?;
            }
            let ref_type = self.get_ref_type(array_type.clone())?;
            *looup.borrow_mut() = ref_type.clone();
            Ok(ref_type)
        } else {
            Err(Error::new(vec![Diagnostic::new(
                "expected to index into an array".to_string(),
                object.source(),
            )]))
        }
    }

    fn visit_call(
        &mut self,
        callee: &'a Rc<Expr>,
        arguments: &'a [Rc<Expr>],
    ) -> Result<Rc<TypeDef>> {
        let callee_type = self.visit_expr(callee)?;
        if let TypeDef::Function {
            parameters,
            return_type,
        } = &*callee_type
        {
            for (argument, para_type) in arguments.iter().zip(parameters) {
                let arg_type = self.visit_expr(argument)?;
                if !arg_type.is_castable_to(&*self.get_ref_type(para_type.clone())?) {
                    Err(Error::new(vec![Diagnostic::new(
                        format!(
                            "argument type '{}' supplied to function does not match parameter type '{}'",
                            arg_type, para_type
                        ),
                        callee.source(),
                    )]))?;
                }
            }
            self.get_ref_type(return_type.clone())
        } else {
            Err(Error::new(vec![Diagnostic::new(
                "expected call a function".to_string(),
                callee.source(),
            )]))
        }
    }

    fn visit_create_array(
        &mut self,
        array_type: &'a TypeCell,
        array_size: &'a Option<Rc<Expr>>,
        fields: &'a [Rc<Expr>],
    ) -> Result<Rc<TypeDef>> {
        if let Some(size_expr) = array_size {
            if !self.visit_expr(size_expr)?.is_integer() {
                Err(Error::new(vec![Diagnostic::new(
                    "expected size of array to be an integer".to_string(),
                    size_expr.source(),
                )]))?;
            }
        }
        let ref_type = self.get_ref_type(array_type.borrow().clone())?;
        *array_type.borrow_mut() = ref_type.clone();

        for field in fields {
            if !self.visit_expr(field)?.is_castable_to(&ref_type) {
                Err(Error::new(vec![Diagnostic::new(
                    "expected size of array to be an integer".to_string(),
                    field.source(),
                )]))?;
            }
        }

        Ok(Rc::new(TypeDef::Array(ref_type)))
    }

    fn visit_create_struct(
        &mut self,
        struct_type: &'a TypeCell,
        fields: &'a [(Identifier, Rc<Expr>)],
    ) -> Result<Rc<TypeDef>> {
        let ref_type = self.get_ref_type(struct_type.borrow().clone())?;

        for (field_name, field_expr) in fields {
            let field_type = self.visit_expr(field_expr)?;
            if !field_type.is_castable_to(&*self.get_member(ref_type.clone(), field_name)?) {
                Err(Error::new(vec![Diagnostic::new(
                    "expected size of array to be an integer".to_string(),
                    field_expr.source(),
                )]))?;
            }
        }
        *struct_type.borrow_mut() = ref_type.clone();
        Ok(ref_type)
    }

    fn visit_if(
        &mut self,
        condition: &'a Rc<Expr>,
        if_branch: &'a Rc<Stmt>,
        else_branch: &'a Option<Rc<Stmt>>,
        expression_type: &'a TypeCell,
    ) -> Result<Rc<TypeDef>> {
        self.visit_expr(condition)?;
        match self.visit_stmt(if_branch) {
            Ok(None) => Ok(self
                .types
                .get("void")
                .expect("expected native type void exists")
                .clone()),
            Ok(Some(if_type)) => {
                if let Some(else_branch) = else_branch {
                    match self.visit_stmt(else_branch) {
                        Ok(Some(else_type)) => {
                            if if_type != else_type {
                                Err(Error::new(vec![Diagnostic::new(
                                    "if branch and else branch have different return types"
                                        .to_string(),
                                    Source::union(&condition.source(), &else_branch.source()),
                                )]))?
                            }
                            *expression_type.borrow_mut() = if_type.clone();
                            Ok(if_type)
                        }
                        Ok(None) => {
                            let void = self
                                .types
                                .get("void")
                                .expect("expected native type void exists")
                                .clone();
                            if if_type != void {
                                Err(Error::new(vec![Diagnostic::new(
                                    "if branch and else branch have different return types"
                                        .to_string(),
                                    Source::union(&condition.source(), &else_branch.source()),
                                )]))?
                            }
                            Ok(void)
                        }

                        Err(e) => Err(e),
                    }
                } else {
                    Err(Error::new(vec![Diagnostic::new(
                        "if expression expected else branch".to_string(),
                        Source::union(&condition.source(), &if_branch.source()),
                    )]))?
                }
            }
            Err(_) => todo!(),
        }
    }

    fn visit_literal(&mut self, value: &Token) -> Result<Rc<TypeDef>> {
        Ok(self
            .types
            .get(match value.get_type() {
                TokenType::Number(_) => "i32",
                TokenType::String(_) => "str",
                TokenType::Bool(_) => "bool",
                _ => unreachable!(),
            })
            .expect(&format!("expected native type '{:?}' exists", value))
            .clone())
    }

    fn visit_variable(
        &mut self,
        name: &Identifier,
        variable_type: &'a TypeCell,
    ) -> Result<Rc<TypeDef>> {
        let real_type = self.lookup_variable(name)?;
        *variable_type.borrow_mut() = real_type.clone();
        Ok(real_type)
    }
}

impl<'a> StmtVisitor<'a> for Resolution<'a> {
    type Output = Result<Option<Rc<TypeDef>>>;

    fn visit_block(&mut self, statements: &'a [Rc<Stmt>]) -> Self::Output {
        let mut errors = Option::None;
        self.begin_scope();
        for stmt in statements {
            if self.return_type.is_some() {
                return Err(Error::new(vec![Diagnostic::new(
                    "function has already returned".to_string(),
                    stmt.source(),
                )]));
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
        initializer: &'a Expr,
    ) -> Self::Output {
        let mut real_type = self.visit_expr(initializer)?;
        if let TypeDef::Lazy(expected_type) = var_type.borrow().as_ref() {
            let ref_type = self.get_type(expected_type)?;
            if real_type.is_castable_to(&ref_type) {
                real_type = ref_type;
            } else {
                Err(Error::new(vec![Diagnostic::new(
                    "assiged a value that does not have the declared type".to_string(),
                    name.get_source(),
                )]))?;
            }
        }
        self.declare_variable(name, real_type.clone())?;
        *var_type.borrow_mut() = real_type;
        Ok(None)
    }

    fn visit_return(&mut self, value: &'a Expr) -> Self::Output {
        self.return_type = Some(self.visit_expr(value)?);
        Ok(None)
    }

    fn visit_break(&mut self) -> Self::Output {
        Ok(None)
    }

    fn visit_while(&mut self, condition: &'a Expr, body: &'a Rc<Stmt>) -> Self::Output {
        self.begin_scope();
        if !self.visit_expr(condition)?.is_bool() {
            Err(Error::new(vec![Diagnostic::new(
                "expected while confition to be boolean".to_string(),
                condition.source(),
            )]))?
        }
        self.visit_stmt(body)?;
        self.end_scope();
        Ok(None)
    }

    fn visit_for(
        &mut self,
        initializer: &'a Rc<Stmt>,
        condition: &'a Expr,
        increment: &'a Expr,
        body: &'a Rc<Stmt>,
    ) -> Self::Output {
        self.begin_scope();
        self.visit_stmt(initializer)?;
        if !self.visit_expr(condition)?.is_bool() {
            Err(Error::new(vec![Diagnostic::new(
                "expected condition in for loop to evaluate to a boolean".to_string(),
                condition.source(),
            )]))?
        }
        // We don't check the return type of the increment expression, because we allow any type
        self.visit_expr(increment)?;
        self.visit_stmt(body)?;
        self.end_scope();
        Ok(None)
    }

    fn visit_expr_stmt(&mut self, expr: &'a Expr) -> Self::Output {
        Ok(Some(self.visit_expr(expr)?))
    }
}

impl<'a> DeclVisitor<'a> for Resolution<'a> {
    type Output = Result<()>;

    fn visit_import(&mut self, _: &[Identifier]) -> Result<()> {
        Ok(())
    }

    fn visit_struct(
        &mut self,
        name: &'a Identifier,
        members: &'a [(Identifier, TypeCell)],
        methods: &'a [Rc<Decl>],
    ) -> Result<()> {
        self.begin_scope();
        let struct_type = self
            .types
            .get(name.get_name())
            .expect("expected to find own struct name when trying to reference self")
            .clone();
        (*self.scopes.last_mut().expect("expected scope"))
            .borrow_mut()
            .insert("self".to_string(), struct_type);
        for (_, member_type) in members {
            let ref_type = self.get_ref_type(member_type.borrow().clone())?;
            *member_type.borrow_mut() = ref_type;
        }
        for method in methods {
            self.visit_decl(method)?;
        }
        self.end_scope();
        Ok(())
    }

    fn visit_procedure(
        &mut self,
        identifier: &'a Identifier,
        return_type: &'a TypeCell,
        params: &'a [(Identifier, TypeCell)],
        body: &'a [Stmt],
        is_extern: bool,
    ) -> Result<()> {
        if self.return_type.is_some() {
            Err(Error::new(vec![Diagnostic::new(
                "return type already exists before body was defined".to_string(),
                identifier.get_source(),
            )]))?;
        }
        let ref_type = self.get_ref_type(return_type.borrow().clone())?;
        *return_type.borrow_mut() = ref_type.clone();

        // When a new body begins, first define all parameters as local variables.
        self.begin_scope();
        for (param_name, param_type) in params {
            let ref_type = self.get_ref_type(param_type.borrow().clone())?;
            self.declare_variable(param_name, ref_type.clone())?;
            *param_type.borrow_mut() = ref_type;
        }
        if is_extern {
            // This function is defined outside of tau, there is no body that we can typecheck.
            self.end_scope();
            return Ok(());
        }

        // Check body. If an error occures, collect errors instead of throwing them directly.
        let mut errors = Option::None;
        for stmt in body {
            if let Err(err) = self.visit_stmt(stmt) {
                errors = err.after(errors);
            }
        }
        if let Some(err) = errors {
            return Err(err);
        }

        // Calculate what was declared as a return type and what we really got.
        let real = self.get_ref_type(
            self.return_type
                .clone()
                .unwrap_or_else(|| self.get_type("void").unwrap()),
        )?;
        if !real.is_castable_to(ref_type.as_ref()) {
            Err(Error::new(vec![Diagnostic::new(
                format!("could not cast {}", real),
                identifier.get_source(),
            )]))?;
        }
        self.return_type = None;
        self.end_scope();
        Ok(())
    }

    fn visit_const(
        &mut self,
        _: &'a Identifier,
        var_type: &'a TypeCell,
        initializer: &'a Expr,
    ) -> Result<()> {
        let ref_type = self.get_ref_type(var_type.borrow().clone())?;
        if !(ref_type.clone() == self.visit_expr(initializer).unwrap()) {
            Err(Error::new(vec![Diagnostic::new(
                "const does not have the declared type".to_string(),
                initializer.source(),
            )]))?;
        }
        *var_type.borrow_mut() = ref_type;
        Ok(())
    }
}
