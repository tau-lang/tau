use std::collections::HashMap;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::ast::DeclVisitor;
use crate::ast::ExprVisitor;
use crate::ast::StmtVisitor;
use crate::ast::{Decl, Expr, Stmt};
use crate::header::Header;
use crate::header::TypeDef;
use crate::lexer::{Token, TokenType};

#[derive(Debug)]
pub struct Resolution<'a> {
    types: HashMap<&'a str, Rc<TypeDef<'a>>>,
    // Current scopes is the deque of all scopes we visited once. The map contains all
    // variable names mapped to its type.
    all_scopes: VecDeque<HashMap<&'a str, Rc<TypeDef<'a>>>>,
    // Current scopes is the deque of the scope we are currently in and all scopes above it.
    current_scopes: VecDeque<HashMap<&'a str, Rc<TypeDef<'a>>>>,
    return_type: Option<Rc<TypeDef<'a>>>,
}

impl<'a> Resolution<'a> {
    pub fn new(header: Header<'a>) -> Resolution<'a> {
        let (types, fields) = header.analysed();
        Resolution {
            types,
            all_scopes: VecDeque::new(),
            current_scopes: VecDeque::from([fields]),
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

    pub fn analysed(
        self,
    ) -> (
        HashMap<&'a str, Rc<TypeDef<'a>>>,
        VecDeque<HashMap<&'a str, Rc<TypeDef<'a>>>>,
    ) {
        (self.types, self.all_scopes)
    }

    fn declare_variable(&mut self, var_name: &'a Token, var_type: Rc<TypeDef<'a>>) {
        assert!(
            self.current_scopes
                .back_mut()
                .expect("expected scope")
                .insert(var_name.identifier(), var_type)
                .is_none()
        );
    }

    fn lookup_variable(&mut self, var_name: &Token) -> Rc<TypeDef<'a>> {
        match self.current_scopes.back() {
            Some(scope) => match scope.get(var_name.identifier()) {
                Some(v) => v.clone(),
                _ => {
                    let origin = self.current_scopes.pop_back().expect("expected scope");
                    let var_type = self.lookup_variable(var_name);
                    self.current_scopes.push_back(origin);
                    var_type
                }
            },
            _ => panic!("could not find variable '{}'", var_name.identifier()),
        }
    }

    fn get_type(&self, name: &str) -> Rc<TypeDef<'a>> {
        let ref_type: Rc<TypeDef<'a>> = self
            .types
            .get(name)
            .expect(
                format!(
                    "expected type name does not exist '{}' in {:#?}",
                    name, self
                )
                .as_str(),
            )
            .clone();
        if let TypeDef::Lazy(lazy_name) = *ref_type {
            self.get_type(lazy_name)
        } else {
            ref_type
        }
    }

    fn get_member(&self, struct_type: Rc<TypeDef<'a>>, member_name: &'a str) -> Rc<TypeDef<'a>> {
        if let TypeDef::Struct { name: _, members } = &*struct_type {
            let ref_type: Rc<TypeDef<'a>> = members
                .get(member_name)
                .expect("expected struct contains field")
                .clone();
            if let TypeDef::Lazy(lazy_name) = *ref_type {
                self.get_type(lazy_name)
            } else {
                ref_type
            }
        } else {
            panic!("expected struct")
        }
    }

    fn begin_scope(&mut self) {
        self.current_scopes.push_back(HashMap::new());
    }

    fn end_scope(&mut self) {
        let old_scope = self.current_scopes.pop_back().expect("expect end scope");
        self.all_scopes.push_front(old_scope);
    }
}

impl<'a> ExprVisitor<'a, Rc<TypeDef<'a>>> for Resolution<'a> {
    fn visit_unary(&mut self, operator: &Token, right: &'a Rc<Expr>) -> Rc<TypeDef<'a>> {
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
    ) -> Rc<TypeDef<'a>> {
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
                // TODO: choose larger number type out of left and right
                r
            }
            TokenType::And | TokenType::Or | TokenType::Xor => {
                assert!(l.is_bool());
                assert!(r.is_bool());
                l
            }
            TokenType::Leq | TokenType::Low | TokenType::Geq | TokenType::Gre => {
                assert!(l.is_number());
                assert!(r.is_number());
                l
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
                // TODO: check if right type can be cast to left
                assert_eq!(l, r);
                l
            }
            _ => unreachable!("{:?}", operator),
        }
    }

    fn visit_get(&mut self, left: &'a Rc<Expr>, right: &'a Token) -> Rc<TypeDef<'a>> {
        let l = self.visit_expr(left);
        let r = right.identifier();
        self.get_member(l, r)
    }

    fn visit_index(&mut self, object: &'a Rc<Expr>, index: &'a Rc<Expr>) -> Rc<TypeDef<'a>> {
        let l = self.visit_expr(object);
        let r = self.visit_expr(index);
        assert!(r.is_integer());
        todo!()
    }

    fn visit_call(
        &mut self,
        callee: &'a Rc<Expr>,
        arguments: &'a Vec<Rc<Expr>>,
    ) -> Rc<TypeDef<'a>> {
        let callee_type = self.visit_expr(callee);
        if let TypeDef::Function {
            name,
            parameters,
            return_type,
        } = &*callee_type
        {
            for argument in arguments {
                // TODO: check argument type
                self.visit_expr(argument);
            }
            return_type.clone()
        } else {
            panic!("expected to call function")
        }
    }

    fn visit_create(
        &mut self,
        struct_name: &'a Token,
        fields: &'a Vec<(Token, Rc<Expr>)>,
    ) -> Rc<TypeDef<'a>> {
        let name = struct_name.identifier();
        let ref_type = if let Some(ref_type) = self.types.get(name) {
            ref_type.clone()
        } else {
            panic!("use of undeclared struct type");
        };

        for (field_name, field_expr) in fields {
            let field_type = self.visit_expr(field_expr);
            assert_eq!(
                self.get_member(ref_type.clone(), field_name.identifier()),
                field_type
            );
        }
        ref_type
    }

    fn visit_if(
        &mut self,
        condition: &'a Rc<Expr>,
        if_branch: &'a Rc<Stmt>,
        else_branch: &'a Option<Rc<Stmt>>,
    ) -> Rc<TypeDef<'a>> {
        self.visit_expr(condition);
        if let Some(if_type) = self.visit_stmt(if_branch) {
            if let Some(branch) = else_branch {
                if let Some(else_type) = self.visit_stmt(branch) {
                    assert_eq!(if_type, else_type);
                    return if_type;
                } else {
                    panic!("return type of if branch does not match else branch");
                }
            } else {
                panic!("expression if expected else branch")
            }
        }
        if let Some(branch) = else_branch {
            if let Some(_) = self.visit_stmt(branch) {
                panic!("non expressive if has a expressive else branch");
            }
        }
        self.types
            .get("void")
            .expect("expected native type void exists")
            .clone()
    }

    fn visit_literal(&mut self, value: &Token) -> Rc<TypeDef<'a>> {
        self.types
            .get(match value.get_type() {
                TokenType::Number(_) => "i32",
                TokenType::String(_) => "str",
                TokenType::Bool(_) => "bool",
                _ => unreachable!(),
            })
            .expect("native type exists")
            .clone()
    }

    fn visit_variable(&mut self, name: &Token) -> Rc<TypeDef<'a>> {
        self.lookup_variable(name)
    }
}

impl<'a> StmtVisitor<'a, Option<Rc<TypeDef<'a>>>> for Resolution<'a> {
    fn visit_block(&mut self, statements: &'a Vec<Rc<Stmt>>) -> Option<Rc<TypeDef<'a>>> {
        self.begin_scope();
        for stmt in statements {
            assert!(self.return_type.is_none(), "function has already returned");
            self.visit_stmt(stmt);
        }
        self.end_scope();
        None
    }

    fn visit_let(&mut self, name: &'a Token, initializer: &'a Expr) -> Option<Rc<TypeDef<'a>>> {
        let var_type = self.visit_expr(initializer);
        self.declare_variable(name, var_type);
        None
    }

    fn visit_return(&mut self, value: &'a Expr) -> Option<Rc<TypeDef<'a>>> {
        self.return_type = Some(self.visit_expr(value));
        None
    }

    fn visit_break(&mut self) -> Option<Rc<TypeDef<'a>>> {
        None
    }

    fn visit_while(&mut self, condition: &'a Expr, body: &'a Rc<Stmt>) -> Option<Rc<TypeDef<'a>>> {
        self.begin_scope();
        assert!(self.visit_expr(condition).is_bool());
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
    ) -> Option<Rc<TypeDef<'a>>> {
        self.begin_scope();
        self.visit_stmt(initializer);
        assert!(self.visit_expr(condition).is_bool());
        // We don't check the return type of the increment expression, because we allow any type
        self.visit_expr(increment);
        self.visit_stmt(body);
        self.end_scope();
        None
    }

    fn visit_expr_stmt(&mut self, expr: &'a Expr) -> Option<Rc<TypeDef<'a>>> {
        Some(self.visit_expr(expr))
    }
}

impl<'a> DeclVisitor<'a, ()> for Resolution<'a> {
    fn visit_import(&mut self, _: &Token) {}

    fn visit_struct(
        &mut self,
        name: &'a Token,
        _: &'a Vec<(Token, Token)>,
        methods: &'a Vec<Rc<Decl>>,
    ) {
        self.begin_scope();
        let struct_type = self
            .types
            .get(name.identifier())
            .expect("expected to find own struct name when trying to reference self")
            .clone();
        self.current_scopes
            .back_mut()
            .expect("expected scope")
            .insert("self", struct_type);
        for method in methods {
            self.visit_decl(method);
        }
        self.end_scope();
    }

    fn visit_function(
        &mut self,
        _: &'a Token,
        return_type: &'a Token,
        params: &'a Vec<(Token, Token)>,
        body: &'a Stmt,
    ) {
        assert!(self.return_type.is_none());
        self.begin_scope();
        for (param_name, param_type) in params {
            self.declare_variable(param_name, self.get_type(param_type.identifier()));
        }
        self.visit_stmt(body);
        assert_eq!(
            self.get_type(return_type.identifier()),
            self.return_type
                .clone()
                .unwrap_or_else(|| { self.get_type("void") })
        );
        self.return_type = None;
        self.end_scope();
    }

    fn visit_const(
        &mut self,
        _: &'a crate::lexer::Token,
        var_type: &'a crate::lexer::Token,
        initializer: &'a crate::ast::Expr,
    ) {
        assert_eq!(
            self.get_type(var_type.identifier()),
            self.visit_expr(initializer)
        );
    }
}
