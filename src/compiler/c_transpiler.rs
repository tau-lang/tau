use std::path::PathBuf;

pub fn replace_extension(path: &str, new_ext: &str) -> String {
    let mut path_buf = PathBuf::from(path);
    path_buf.set_extension(new_ext);
    path_buf.to_string_lossy().into_owned()
}

pub mod c {
    use crate::lexer::TokenType;
    use crate::{
        ast::{Decl, DeclVisitor, Expr, ExprVisitor, Stmt, StmtVisitor},
        compiler::Generator,
        lexer::Token,
    };
    use std::fs::File;
    use std::io::Error;
    use std::io::prelude::*;
    use std::rc::Rc;

    pub fn to_c_type(tau_type: &str) -> &str {
        match tau_type {
            "u8" => "unsigned short int",
            "u16" => "unsigned short int",
            "u32" => "unsigned int",
            "u64" => "unsigned long int",
            "i8" => "short int",
            "i16" => "short int",
            "i32" => "int",
            "i64" => "long int",
            "f32" => "float",
            "f64" => "double",
            _ => tau_type,
        }
    }

    pub struct CHeaderGenerator;

    impl CHeaderGenerator {
        fn visit_method(
            &mut self,
            self_type: &'_ Token,
            name: &'_ Token,
            return_type: &'_ Option<Token>,
            params: &'_ [(Token, Token)],
        ) -> String {
            let mut builder = String::from("\nextern ");
            builder.push_str(if let Some(value) = return_type {
                to_c_type(value.identifier())
            } else {
                "void"
            });
            builder.push_str(" ");
            builder.push_str(self_type.identifier());
            builder.push_str("$");
            builder.push_str(name.identifier());
            builder.push_str("(");
            builder.push_str(self_type.identifier());
            builder.push_str(" ");
            builder.push_str("self");
            for (param_name, param_type) in params {
                builder.push_str(", ");
                builder.push_str(to_c_type(param_type.identifier()));
                builder.push_str(" ");
                builder.push_str(param_name.identifier());
            }
            builder.push_str(");\n");
            builder
        }
    }

    impl Generator for CHeaderGenerator {
        fn generate(&mut self, declarations: &[Decl], output: &mut File) -> Result<(), Error> {
            let mut builder = String::with_capacity(255);
            for decl in declarations {
                builder.push_str(&self.visit_decl(decl));
            }
            write!(output, "{}", builder)
        }
    }

    impl DeclVisitor<'_, String> for CHeaderGenerator {
        fn visit_import(&mut self, name: &'_ Token) -> String {
            let mut builder = String::from("#include <");
            builder.push_str(name.identifier());
            builder.push_str(">\n");
            builder
        }

        fn visit_struct(
            &mut self,
            name: &'_ Token,
            fields: &'_ [(Token, Token)],
            methods: &'_ [Rc<Decl>],
        ) -> String {
            let mut builder = String::from("\ntypedef struct ");
            builder.push_str(name.identifier());
            builder.push_str(" {\n");
            for (field_name, field_type) in fields {
                builder.push_str("\t");
                builder.push_str(to_c_type(field_type.identifier()));
                builder.push_str(" ");
                builder.push_str(field_name.identifier());
                builder.push_str(";\n");
            }
            builder.push_str("} ");
            builder.push_str(name.identifier());
            builder.push_str(";\n");
            let self_type = name;
            for method in methods {
                if let Decl::Function {
                    name,
                    return_type,
                    params,
                    body: _,
                } = &**method
                {
                    builder.push_str(&self.visit_method(self_type, name, return_type, params));
                } else {
                    panic!("expected method");
                }
            }
            builder
        }

        fn visit_function(
            &mut self,
            name: &'_ Token,
            return_type: &'_ Option<Token>,
            params: &'_ [(Token, Token)],
            _: &'_ [Stmt],
        ) -> String {
            let mut builder = String::from("\nextern ");
            builder.push_str(if let Some(value) = return_type {
                to_c_type(value.identifier())
            } else {
                "void"
            });
            builder.push_str(" ");
            builder.push_str(name.identifier());
            builder.push_str("(");
            if params.len() > 0 {
                let (first_name, first_type) = params.get(0).unwrap();
                builder.push_str(to_c_type(first_type.identifier()));
                builder.push_str(" ");
                builder.push_str(first_name.identifier());
                for (param_name, param_type) in &params[1..] {
                    builder.push_str(", ");
                    builder.push_str(to_c_type(param_type.identifier()));
                    builder.push_str(" ");
                    builder.push_str(param_name.identifier());
                }
            }
            builder.push_str(");\n");
            builder
        }

        fn visit_const(&mut self, name: &'_ Token, var_type: &'_ Token, _: &'_ Expr) -> String {
            let mut builder = String::from("\nextern const ");
            builder.push_str(to_c_type(var_type.identifier()));
            builder.push_str(" ");
            builder.push_str(name.identifier());
            builder.push_str(";\n");
            builder
        }
    }

    pub struct CCodeGenerator;

    impl CCodeGenerator {
        fn visit_method(
            &mut self,
            self_type: &'_ Token,
            name: &'_ Token,
            return_type: &'_ Option<Token>,
            params: &'_ [(Token, Token)],
            body: &'_ [Stmt],
        ) -> String {
            let mut builder = String::with_capacity(255);
            builder.push_str(if let Some(value) = return_type {
                to_c_type(value.identifier())
            } else {
                "void"
            });
            builder.push_str(" ");
            builder.push_str(self_type.identifier());
            builder.push_str("$");
            builder.push_str(name.identifier());
            builder.push_str("(");
            builder.push_str(self_type.identifier());
            builder.push_str(" ");
            builder.push_str("self");
            for (param_name, param_type) in params {
                builder.push_str(", ");
                builder.push_str(to_c_type(param_type.identifier()));
                builder.push_str(" ");
                builder.push_str(param_name.identifier());
            }
            builder.push_str(") {\n");
            for stmt in body {
                builder.push_str(&self.visit_stmt(stmt));
            }
            builder.push_str("}\n");
            builder
        }
    }

    impl Generator for CCodeGenerator {
        fn generate(&mut self, ast: &[Decl], output: &mut File) -> Result<(), Error> {
            let mut builder = String::with_capacity(255);
            for decl in ast {
                builder.push_str(&self.visit_decl(decl));
            }
            write!(output, "{}", builder)
        }
    }

    impl ExprVisitor<'_, String> for CCodeGenerator {
        fn visit_unary(&mut self, operator: &'_ Token, right: &'_ Rc<Expr>) -> String {
            match operator.get_type() {
                TokenType::Add => format!("+{}", self.visit_expr(right)),
                TokenType::Sub => format!("-{}", self.visit_expr(right)),
                TokenType::Not => format!("!{}", self.visit_expr(right)),
                _ => todo!(),
            }
        }

        fn visit_binary(
            &mut self,
            left: &'_ Rc<Expr>,
            operator: &'_ Token,
            right: &'_ Rc<Expr>,
        ) -> String {
            match operator.get_type() {
                TokenType::Add => format!("{} + {}", self.visit_expr(left), self.visit_expr(right)),
                TokenType::Sub => format!("{} - {}", self.visit_expr(left), self.visit_expr(right)),
                TokenType::Mul => format!("{} * {}", self.visit_expr(left), self.visit_expr(right)),
                TokenType::Div => format!("{} / {}", self.visit_expr(left), self.visit_expr(right)),
                TokenType::Eq => format!("{} == {}", self.visit_expr(left), self.visit_expr(right)),
                TokenType::Neq => {
                    format!("{} != {}", self.visit_expr(left), self.visit_expr(right))
                }
                TokenType::Low => format!("{} < {}", self.visit_expr(left), self.visit_expr(right)),
                TokenType::Leq => {
                    format!("{} <= {}", self.visit_expr(left), self.visit_expr(right))
                }
                TokenType::Gre => format!("{} > {}", self.visit_expr(left), self.visit_expr(right)),
                TokenType::Geq => {
                    format!("{} >= {}", self.visit_expr(left), self.visit_expr(right))
                }
                TokenType::And => {
                    format!("{} && {}", self.visit_expr(left), self.visit_expr(right))
                }
                TokenType::Or => format!("{} || {}", self.visit_expr(left), self.visit_expr(right)),
                TokenType::Xor => format!("{} | {}", self.visit_expr(left), self.visit_expr(right)),
                TokenType::Set => format!("{} = {}", self.visit_expr(left), self.visit_expr(right)),
                TokenType::SetAdd => {
                    format!("{} += {}", self.visit_expr(left), self.visit_expr(right))
                }
                TokenType::SetSub => {
                    format!("{} -= {}", self.visit_expr(left), self.visit_expr(right))
                }
                TokenType::SetMul => {
                    format!("{} *= {}", self.visit_expr(left), self.visit_expr(right))
                }
                TokenType::SetDiv => {
                    format!("{} /= {}", self.visit_expr(left), self.visit_expr(right))
                }
                _ => todo!("{:?}", operator),
            }
        }

        fn visit_get(&mut self, left: &'_ Rc<Expr>, right: &'_ Token) -> String {
            format!("{}.{}", self.visit_expr(left), right.identifier())
        }

        fn visit_index(&mut self, object: &'_ Rc<Expr>, index: &'_ Rc<Expr>) -> String {
            format!("{}[{}]", self.visit_expr(object), self.visit_expr(index))
        }

        fn visit_call(&mut self, callee: &'_ Rc<Expr>, arguments: &'_ [Rc<Expr>]) -> String {
            let mut builder = self.visit_expr(callee);
            builder.push_str("(");
            if arguments.len() > 0 {
                let first_arg = arguments.get(0).unwrap();
                builder.push_str(&self.visit_expr(first_arg));
                for arg in &arguments[1..] {
                    builder.push_str(", ");
                    builder.push_str(&self.visit_expr(arg));
                }
            }
            builder.push_str(")");
            builder
        }

        fn visit_create_array(
            &mut self,
            array_type: &'_ Token,
            array_size: &'_ Option<Rc<Expr>>,
            fields: &'_ [Rc<Expr>],
        ) -> String {
            todo!()
        }

        fn visit_create_struct(&mut self, _: &'_ Token, fields: &'_ [(Token, Rc<Expr>)]) -> String {
            let mut builder = String::from("{");
            for (field_name, field_init) in fields {
                builder.push_str(" .");
                builder.push_str(field_name.identifier());
                builder.push_str(" = ");
                builder.push_str(&self.visit_expr(field_init));
                builder.push_str(",")
            }
            builder.push_str("}");
            builder
        }

        fn visit_if(
            &mut self,
            condition: &'_ Rc<Expr>,
            if_branch: &'_ Rc<Stmt>,
            else_branch: &'_ Option<Rc<Stmt>>,
        ) -> String {
            let mut builder = String::from("if (");
            builder.push_str(&self.visit_expr(condition));
            builder.push_str(") ");
            builder.push_str(&self.visit_stmt(if_branch));
            if let Some(branch) = else_branch {
                builder.push_str("else ");
                builder.push_str(&self.visit_stmt(branch))
            }
            builder
        }

        fn visit_literal(&mut self, value: &'_ Token) -> String {
            match value.get_type() {
                TokenType::Bool(value) => value.to_string(),
                TokenType::Number(value) => value.to_string(),
                TokenType::String(content) => format!("\"{}\"", content),
                _ => todo!(),
            }
        }

        fn visit_variable(&mut self, name: &'_ Token) -> String {
            String::from(name.identifier())
        }
    }

    impl StmtVisitor<'_, String> for CCodeGenerator {
        fn visit_block(&mut self, statements: &'_ [Rc<Stmt>]) -> String {
            let mut builder = String::from("{\n");
            for stmt in statements {
                builder.push_str(&self.visit_stmt(stmt));
            }
            builder.push_str("}\n");
            builder
        }

        fn visit_let(
            &mut self,
            name: &'_ Token,
            var_type: &'_ Option<Token>,
            initializer: &'_ Expr,
        ) -> String {
            let mut builder = String::from(name.identifier());
            builder.push_str(" = ");
            builder.push_str(&self.visit_expr(initializer));
            builder.push_str(";\n");
            builder
        }

        fn visit_return(&mut self, value: &'_ Expr) -> String {
            let mut builder = String::from("return ");
            builder.push_str(&self.visit_expr(value));
            builder.push_str(";\n");
            builder
        }

        fn visit_break(&mut self) -> String {
            String::from("break;\n")
        }

        fn visit_while(&mut self, condition: &'_ Expr, body: &'_ Rc<Stmt>) -> String {
            let mut builder = String::from("while (");
            builder.push_str(&self.visit_expr(condition));
            builder.push_str(") ");
            builder.push_str(&self.visit_stmt(body));
            builder
        }

        fn visit_for(
            &mut self,
            initializer: &'_ Rc<Stmt>,
            condition: &'_ Expr,
            increment: &'_ Expr,
            body: &'_ Rc<Stmt>,
        ) -> String {
            let mut builder = String::from("for (");
            builder.push_str(&self.visit_stmt(initializer));
            builder.push_str(&self.visit_expr(condition));
            builder.push_str(";");
            builder.push_str(&self.visit_expr(increment));
            builder.push_str(") ");
            builder.push_str(&self.visit_stmt(body));
            builder
        }

        fn visit_expr_stmt(&mut self, expr: &'_ Expr) -> String {
            let mut builder = self.visit_expr(expr);
            builder.push_str(";\n");
            builder
        }
    }

    impl DeclVisitor<'_, String> for CCodeGenerator {
        fn visit_import(&mut self, _: &'_ Token) -> String {
            String::from("")
        }

        fn visit_struct(
            &mut self,
            name: &'_ Token,
            _: &'_ [(Token, Token)],
            methods: &'_ [Rc<Decl>],
        ) -> String {
            let mut builder = String::with_capacity(255);
            let self_type = name;
            for method in methods {
                if let Decl::Function {
                    name,
                    return_type,
                    params,
                    body,
                } = &**method
                {
                    builder.push_str(&self.visit_method(self_type, name, return_type, params, body))
                } else {
                    panic!("expected method")
                }
            }
            builder
        }

        fn visit_function(
            &mut self,
            name: &'_ Token,
            return_type: &'_ Option<Token>,
            params: &'_ [(Token, Token)],
            body: &'_ [Stmt],
        ) -> String {
            let mut builder = String::with_capacity(255);
            if let Some(rtype) = return_type {
                builder.push_str(to_c_type(rtype.identifier()));
            } else {
                builder.push_str("void");
            }
            builder.push_str(" ");
            builder.push_str(name.identifier());
            builder.push_str("(");
            if params.len() > 0 {
                let (first_name, first_type) = params.get(0).unwrap();
                builder.push_str(to_c_type(first_type.identifier()));
                builder.push_str(" ");
                builder.push_str(first_name.identifier());
                for (param_name, param_type) in &params[1..] {
                    builder.push_str(", ");
                    builder.push_str(to_c_type(param_type.identifier()));
                    builder.push_str(" ");
                    builder.push_str(param_name.identifier());
                }
            }
            builder.push_str(") {\n");
            for stmt in body {
                builder.push_str(&self.visit_stmt(stmt));
            }
            builder.push_str("}\n");
            builder
        }

        fn visit_const(
            &mut self,
            name: &'_ Token,
            var_type: &'_ Token,
            initializer: &'_ Expr,
        ) -> String {
            let mut builder = String::from(to_c_type(var_type.identifier()));
            builder.push_str(" ");
            builder.push_str(to_c_type(name.identifier()));
            builder.push_str(" = ");
            builder.push_str(&self.visit_expr(initializer));
            builder.push_str(";");
            builder
        }
    }
}
