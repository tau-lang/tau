use crate::{
    ast::{Decl, DeclVisitor, Expr, ExprVisitor, Identifier, Stmt, StmtVisitor},
    lexer::{Token, TokenType},
    typing::TypeCell,
    typing::{TypeDef, TypeNames},
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

    pub fn resolve(mut self, declarations: &'a Vec<Decl>) -> Self {
        for decl in declarations {
            self.visit_decl(decl)
        }
        // When we create a new resolution, we already start in the global scope.
        // After visiting all declarations, we need to end the global scope and
        // push it to all scopes.
        self.end_scope();
        self
    }

    pub fn analysed(self) -> Vec<Rc<RefCell<TypeNames>>> {
        self.scopes
    }

    fn declare_variable(&mut self, var_name: &str, var_type: Rc<TypeDef>) {
        assert!(
            (*self.scopes.last_mut().expect("expected scope"))
                .borrow_mut()
                .insert(var_name.to_string(), var_type)
                .is_none()
        );
    }

    fn lookup_variable(&mut self, var_name: &str) -> Rc<TypeDef> {
        if let Some(scope) = self.scopes.last() {
            if let Some(v) = scope.borrow().get(var_name) {
                v.clone()
            } else {
                let origin = self.scopes.pop().expect("expected scope");
                let var_type = self.lookup_variable(var_name);
                self.scopes.push(origin);
                var_type
            }
        } else {
            panic!("could not find variable '{}'", var_name);
        }
    }

    fn get_type(&self, name: &str) -> Rc<TypeDef> {
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

    fn get_ref_type(&self, ref_type: Rc<TypeDef>) -> Rc<TypeDef> {
        if let TypeDef::Lazy(lazy_name) = ref_type.as_ref() {
            self.get_type(lazy_name)
        } else {
            ref_type
        }
    }

    fn get_member(&self, lookup: Rc<TypeDef>, member_name: &'a str) -> Rc<TypeDef> {
        if let TypeDef::Struct { name: _, members } = lookup.as_ref() {
            let ref_type: Rc<TypeDef> = members
                .get(member_name)
                .expect("expected struct contains field")
                .clone();
            if let TypeDef::Lazy(lazy_name) = ref_type.as_ref() {
                self.get_type(lazy_name)
            } else {
                ref_type
            }
        } else if let TypeDef::Module { types: _, fields } = lookup.as_ref() {
            let ref_type = fields
                .get(member_name)
                .expect("expected modue contains field")
                .clone();
            if let TypeDef::Lazy(lazy_name) = ref_type.as_ref() {
                self.get_type(lazy_name)
            } else {
                ref_type
            }
        } else {
            panic!("expected struct")
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

impl<'a> ExprVisitor<'a, Rc<TypeDef>> for Resolution<'a> {
    fn visit_unary(&mut self, operator: &Token, right: &'a Rc<Expr>) -> Rc<TypeDef> {
        let r = self.visit_expr(right);
        match operator.get_type() {
            TokenType::Not => {
                assert!(r.is_bool());
                r
            }
            TokenType::Add | TokenType::Sub => {
                assert!(r.is_number());
                r
            }
            _ => unreachable!("{:?}", operator),
        }
    }

    fn visit_binary(
        &mut self,
        left: &'a Rc<Expr>,
        operator: &Token,
        right: &'a Rc<Expr>,
    ) -> Rc<TypeDef> {
        let l = self.visit_expr(left);
        let r = self.visit_expr(right);
        match operator.get_type() {
            TokenType::Add
            | TokenType::Sub
            | TokenType::Mul
            | TokenType::Div
            | TokenType::SetAdd
            | TokenType::SetSub
            | TokenType::SetMul
            | TokenType::SetDiv => {
                assert!(l.is_number());
                assert!(r.is_number());
                if l.is_castable_to(&r) {
                    return r;
                } else if r.is_castable_to(&l) {
                    return l;
                }
                panic!("number types of left and right side do not match");
            }
            TokenType::And | TokenType::Or | TokenType::Xor => {
                assert!(l.is_bool());
                assert!(r.is_bool());
                l
            }
            TokenType::Leq | TokenType::Low | TokenType::Geq | TokenType::Gre => {
                assert!(l.is_number());
                assert!(r.is_number());
                self.get_type("bool")
            }
            TokenType::Eq | TokenType::Neq => {
                if l.is_number() {
                    assert!(r.is_number());
                } else {
                    assert_eq!(l, r);
                }
                l
            }
            TokenType::Set => {
                if !r.is_castable_to(&l) {
                    panic!("can not cast right side of set expression to expected var type")
                }
                l
            }
            _ => unreachable!("{:?}", operator),
        }
    }

    fn visit_get(
        &mut self,
        left: &'a Rc<Expr>,
        right: &'a Identifier,
        lookup: &'a TypeCell,
    ) -> Rc<TypeDef> {
        let l = self.visit_expr(left);
        *lookup.borrow_mut() = l.clone();
        let r = right.get_name();
        self.get_member(l, r)
    }

    fn visit_index(
        &mut self,
        object: &'a Rc<Expr>,
        index: &'a Rc<Expr>,
        looup: &'a TypeCell,
    ) -> Rc<TypeDef> {
        let l = self.visit_expr(object);
        if let TypeDef::Array(array_type) = &*l {
            let r = self.visit_expr(index);
            assert!(r.is_integer());
            let ref_type = self.get_ref_type(array_type.clone());
            *looup.borrow_mut() = ref_type.clone();
            ref_type
        } else {
            panic!("expected to index array")
        }
    }

    fn visit_call(&mut self, callee: &'a Rc<Expr>, arguments: &'a [Rc<Expr>]) -> Rc<TypeDef> {
        let callee_type = self.visit_expr(callee);
        if let TypeDef::Function {
            parameters,
            return_type,
        } = &*callee_type
        {
            for (argument, para_type) in arguments.iter().zip(parameters) {
                let arg_type = self.visit_expr(argument);
                assert!(
                    arg_type.is_castable_to(&self.get_ref_type(para_type.clone())),
                    "argument type '{}' supplied to function does not match parameter type '{}'",
                    arg_type,
                    para_type
                )
            }
            self.get_ref_type(return_type.clone())
        } else {
            panic!("expected to call function")
        }
    }

    fn visit_create_array(
        &mut self,
        array_type: &'a TypeCell,
        array_size: &'a Option<Rc<Expr>>,
        fields: &'a [Rc<Expr>],
    ) -> Rc<TypeDef> {
        if let Some(size_expr) = array_size {
            assert!(self.visit_expr(size_expr).is_integer());
        }
        let ref_type = self.get_ref_type(array_type.borrow().clone());
        *array_type.borrow_mut() = ref_type.clone();

        for field in fields {
            assert!(self.visit_expr(field).is_castable_to(&ref_type))
        }

        Rc::new(TypeDef::Array(ref_type))
    }

    fn visit_create_struct(
        &mut self,
        struct_type: &'a TypeCell,
        fields: &'a [(Identifier, Rc<Expr>)],
    ) -> Rc<TypeDef> {
        let ref_type = self.get_ref_type(struct_type.borrow().clone());

        for (field_name, field_expr) in fields {
            let field_type = self.visit_expr(field_expr);
            assert!(
                field_type
                    .is_castable_to(&self.get_member(ref_type.clone(), field_name.get_name()))
            );
        }
        *struct_type.borrow_mut() = ref_type.clone();
        ref_type
    }

    fn visit_if(
        &mut self,
        condition: &'a Rc<Expr>,
        if_branch: &'a Rc<Stmt>,
        else_branch: &'a Option<Rc<Stmt>>,
        expression_type: &'a TypeCell,
    ) -> Rc<TypeDef> {
        self.visit_expr(condition);
        if let Some(if_type) = self.visit_stmt(if_branch) {
            if let Some(branch) = else_branch {
                if let Some(else_type) = self.visit_stmt(branch) {
                    assert_eq!(if_type, else_type);
                    *expression_type.borrow_mut() = if_type.clone();
                    return if_type;
                } else {
                    panic!("return type of if branch does not match else branch");
                }
            } else {
                panic!("expression if expected else branch")
            }
        }
        if let Some(branch) = else_branch {
            if self.visit_stmt(branch).is_some() {
                panic!("non expressive if has a expressive else branch");
            }
        }
        self.types
            .get("void")
            .expect("expected native type void exists")
            .clone()
    }

    fn visit_literal(&mut self, value: &Token) -> Rc<TypeDef> {
        self.types
            .get(match value.get_type() {
                TokenType::Number(_) => "i32",
                TokenType::String(_) => "str",
                TokenType::Bool(_) => "bool",
                _ => unreachable!(),
            })
            .expect(&format!("expected native type '{:?}' exists", value))
            .clone()
    }

    fn visit_variable(&mut self, name: &Identifier, variable_type: &'a TypeCell) -> Rc<TypeDef> {
        let real_type = self.lookup_variable(name.get_name());
        *variable_type.borrow_mut() = real_type.clone();
        real_type
    }
}

impl<'a> StmtVisitor<'a, Option<Rc<TypeDef>>> for Resolution<'a> {
    fn visit_block(&mut self, statements: &'a [Rc<Stmt>]) -> Option<Rc<TypeDef>> {
        self.begin_scope();
        for stmt in statements {
            assert!(self.return_type.is_none(), "function has already returned");
            self.visit_stmt(stmt);
        }
        self.end_scope();
        None
    }

    fn visit_let(
        &mut self,
        name: &'a Identifier,
        var_type: &'a TypeCell,
        initializer: &'a Expr,
    ) -> Option<Rc<TypeDef>> {
        let mut real_type = self.visit_expr(initializer);
        if let TypeDef::Lazy(expected_type) = var_type.borrow().as_ref() {
            let ref_type = self.get_type(expected_type);
            if real_type.is_castable_to(&ref_type) {
                real_type = ref_type;
            } else {
                panic!();
            }
        }
        self.declare_variable(name.get_name(), real_type.clone());
        *var_type.borrow_mut() = real_type;
        None
    }

    fn visit_return(&mut self, value: &'a Expr) -> Option<Rc<TypeDef>> {
        self.return_type = Some(self.visit_expr(value));
        None
    }

    fn visit_break(&mut self) -> Option<Rc<TypeDef>> {
        None
    }

    fn visit_while(&mut self, condition: &'a Expr, body: &'a Rc<Stmt>) -> Option<Rc<TypeDef>> {
        self.begin_scope();
        assert!(
            self.visit_expr(condition).is_bool(),
            "expected while condition is boolean"
        );
        self.visit_stmt(body);
        self.end_scope();
        None
    }

    fn visit_for(
        &mut self,
        initializer: &'a Rc<Stmt>,
        condition: &'a Expr,
        increment: &'a Expr,
        body: &'a Rc<Stmt>,
    ) -> Option<Rc<TypeDef>> {
        self.begin_scope();
        self.visit_stmt(initializer);
        assert!(self.visit_expr(condition).is_bool());
        // We don't check the return type of the increment expression, because we allow any type
        self.visit_expr(increment);
        self.visit_stmt(body);
        self.end_scope();
        None
    }

    fn visit_expr_stmt(&mut self, expr: &'a Expr) -> Option<Rc<TypeDef>> {
        Some(self.visit_expr(expr))
    }
}

impl<'a> DeclVisitor<'a, ()> for Resolution<'a> {
    fn visit_import(&mut self, _: &[Identifier]) {}

    fn visit_struct(
        &mut self,
        name: &'a Identifier,
        members: &'a [(Identifier, TypeCell)],
        methods: &'a [Rc<Decl>],
    ) {
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
            let ref_type = self.get_ref_type(member_type.borrow().clone());
            *member_type.borrow_mut() = ref_type;
        }
        for method in methods {
            self.visit_decl(method);
        }
        self.end_scope();
    }

    fn visit_function(
        &mut self,
        _: &'a Identifier,
        return_type: &'a TypeCell,
        params: &'a [(Identifier, TypeCell)],
        body: &'a [Stmt],
        is_extern: bool,
    ) {
        assert!(self.return_type.is_none());
        let ref_type = self.get_ref_type(return_type.borrow().clone());
        *return_type.borrow_mut() = ref_type.clone();
        self.begin_scope();
        for (param_name, param_type) in params {
            let ref_type = self.get_ref_type(param_type.borrow().clone());
            self.declare_variable(param_name.get_name(), ref_type.clone());
            *param_type.borrow_mut() = ref_type;
        }
        if is_extern {
            // This function is defined outside of tau, there is no body that we can typecheck.
            self.end_scope();
            return;
        }
        for stmt in body {
            self.visit_stmt(stmt);
        }
        let real = self.get_ref_type(
            self.return_type
                .clone()
                .unwrap_or_else(|| self.get_type("void")),
        );
        assert!(
            real.is_castable_to(ref_type.as_ref()),
            "could not cast {}",
            real
        );

        self.return_type = None;
        self.end_scope();
    }

    fn visit_const(&mut self, _: &'a Identifier, var_type: &'a TypeCell, initializer: &'a Expr) {
        let ref_type = self.get_ref_type(var_type.borrow().clone());
        assert_eq!(ref_type.clone(), self.visit_expr(initializer));
        *var_type.borrow_mut() = ref_type;
    }
}
