use crate::{source_code::CSourceCode, visit_vec};
use std::{boxed::Box, fs::File, io::prelude::*};
use tau_core::{
    ast::{
        TypedDecl,
        declaration::{DeclFunc, DeclStruct, DeclVisitor},
        expression::{Expr, TypedExprVisitor},
        identifier::Identifier,
        origin::{BinaryOp, ExprOrigin, Literal, UnaryOp},
        statement::{Stmt, StmtKind, StmtVisitor},
    },
    error,
    generator::Generator,
    lexer::Source,
    type_::Typed,
};

pub struct CCodeGenerator {
    indent: usize,
}

impl<'a> CCodeGenerator {
    pub fn new() -> CCodeGenerator {
        CCodeGenerator { indent: 0 }
    }

    fn begin_scope(&mut self) {
        self.indent += 1;
    }

    fn end_scope(&mut self) {
        self.indent -= 1;
    }
}

impl<'a> Generator<'a> for CCodeGenerator {
    fn generate(&mut self, ast: &'a [TypedDecl], output: &std::path::Path) -> error::Result<()> {
        // TODO:
        let mut file = File::create(output).unwrap();
        let mut path_buf = output.to_path_buf();
        path_buf.set_extension("h");
        let header_file = path_buf
            .file_name()
            .expect("expect path has filename")
            .to_str()
            .expect("expect filename is UTF-8");

        let declarations = visit_vec(
            &ast.into_iter()
                .map(|decl| self.visit_decl(decl))
                .filter(|op| op.is_some())
                .map(|op| op.expect("Should only contain some."))
                .collect::<Vec<CSourceCode>>(),
            |decl| format!("{}", decl),
            "\n\n",
        );

        write!(file, "#include \"{header_file}\"\n\n{declarations}\n").unwrap();
        Ok(())
    }
}

impl<'a> TypedExprVisitor<'a> for CCodeGenerator {
    type Type = Typed;
    type Output = CSourceCode;

    fn visit_unary(
        &mut self,
        operator: &ExprOrigin<UnaryOp>,
        right: &'a Expr<Self::Type>,
        _: &Self::Type,
    ) -> CSourceCode {
        match operator.kind() {
            UnaryOp::Neg => format!("-{}", self.visit_expr(right)),
            UnaryOp::Not => format!("!{}", self.visit_expr(right)),
            UnaryOp::Ref => format!("(&{})", self.visit_expr(right)),
            UnaryOp::Deref => format!("(*{})", self.visit_expr(right)),
        }
        .into()
    }

    fn visit_binary(
        &mut self,
        left: &'a Expr<Self::Type>,
        operator: &ExprOrigin<BinaryOp>,
        right: &'a Expr<Self::Type>,
        _: &Self::Type,
    ) -> CSourceCode {
        let mut render_op =
            |op: &str| format!("{} {op} {}", self.visit_expr(left), self.visit_expr(right));
        match operator.kind() {
            BinaryOp::Add => render_op("+"),
            BinaryOp::Sub => render_op("-"),
            BinaryOp::Mul => render_op("*"),
            BinaryOp::Div => render_op("/"),
            BinaryOp::Mod => render_op("%"),

            BinaryOp::Eq => render_op("=="),
            BinaryOp::Neq => render_op("!="),

            BinaryOp::Low => render_op("<"),
            BinaryOp::Leq => render_op("<="),
            BinaryOp::Gre => render_op(">"),
            BinaryOp::Geq => render_op(">="),

            BinaryOp::And => render_op("&&"),
            BinaryOp::Or => render_op("||"),

            BinaryOp::Set => render_op("="),
            BinaryOp::SetAdd => render_op("+="),
            BinaryOp::SetSub => render_op("-="),
            BinaryOp::SetMul => render_op("*="),
            BinaryOp::SetDiv => render_op("/="),
        }
        .into()
    }

    fn visit_get(
        &mut self,
        left: &'a Expr<Self::Type>,
        right: &Identifier,
        _: &Self::Type,
    ) -> CSourceCode {
        let left = self.visit_expr(left);
        let right = right.name();
        format!("{}.{}", left, right).into()
    }

    fn visit_index(
        &mut self,
        object: &'a Expr<Self::Type>,
        index: &'a Expr<Self::Type>,
        _: &'a Source,
        _: &Self::Type,
    ) -> CSourceCode {
        let object = self.visit_expr(object);
        let index = self.visit_expr(index);
        format!("{object}[{index}]").into()
    }

    fn visit_call(
        &mut self,
        callee: &'a Expr<Self::Type>,
        arguments: &'a [Expr<Self::Type>],
        _: &'a Source,
        _: &Self::Type,
    ) -> CSourceCode {
        let callee = self.visit_expr(callee);
        let arguments = arguments
            .iter()
            .map(|x| format!("{}", self.visit_expr(x)))
            .collect::<Vec<String>>()
            .join(", ");
        format!("{callee}({arguments})").into()
    }

    fn visit_create_array(
        &mut self,
        fields: &'a [Expr<Self::Type>],
        _: &'a Source,
        type_: &Self::Type,
    ) -> CSourceCode {
        let fields = visit_vec(fields, |x| format!("{}", self.visit_expr(x)), ", ");
        let type_ = CSourceCode::from(type_);
        format!("{type_}{{{fields}}}").into()
    }

    fn visit_create_struct(
        &mut self,
        fields: &'a [(Identifier, Expr<Self::Type>)],
        _: &'a Source,
        type_: &Self::Type,
    ) -> CSourceCode {
        let type_: CSourceCode = type_.as_ref().into();
        let fields = visit_vec(
            fields,
            |(field_name, field_init)| {
                format!(
                    ".{} = {}",
                    CSourceCode::from(field_name),
                    self.visit_expr(field_init)
                )
            },
            ", ",
        );
        format!("({type_}) {{{fields}}}").into()
    }

    fn visit_if(
        &mut self,
        condition: &'a Expr<Self::Type>,
        if_branch: &'a Box<Stmt<Self::Type>>,
        else_branch: &'a Option<Box<Stmt<Self::Type>>>,
        _: &'a Source,
        type_: &Self::Type,
    ) -> CSourceCode {
        let condition = self.visit_expr(condition);
        if type_.as_ref().is_void() {
            let if_branch = self.visit_stmt(if_branch);
            if let Some(else_branch) = else_branch {
                let else_branch = self.visit_stmt(else_branch);
                format!("if ({condition}) {{ {if_branch} }} else {{ {else_branch} }}").into()
            } else {
                format!("if ({condition}) {{ {if_branch} }}").into()
            }
        } else {
            let stmt2expr = |stmt: &'a Stmt<Self::Type>| match stmt.kind() {
                StmtKind::ExprStmt(expr) => expr,
                _ => panic!("Could not convert statement to expression"),
            };

            let if_branch = self.visit_expr(stmt2expr(&if_branch.as_ref()));
            let else_branch = self.visit_expr(stmt2expr(
                &else_branch
                    .as_ref()
                    .expect("expected else expression exists"),
            ));
            format!("{condition} ? {if_branch} : {else_branch}").into()
        }
    }

    fn visit_literal(&mut self, value: &ExprOrigin<Literal>, _: &Self::Type) -> CSourceCode {
        match value.kind() {
            Literal::Bool(value) => value.to_string(),
            Literal::Int(value) => value.to_string(),
            Literal::Float(value) => value.to_string(),
            Literal::String(content) => format!("\"{}\"", content),
        }
        .into()
    }

    fn visit_variable(&mut self, name: &Identifier, _: &Self::Type) -> CSourceCode {
        name.into()
    }
}

impl<'a> StmtVisitor<'a> for CCodeGenerator {
    type Type = Typed;
    type Output = CSourceCode;

    fn visit_stmt(&mut self, stmt: &'a Stmt<Self::Type>) -> Self::Output {
        let code = match stmt.kind() {
            StmtKind::Block { statements } => self.visit_block(statements),
            StmtKind::Let {
                name,
                declared,
                initializer,
            } => self.visit_let(name, declared, initializer),
            StmtKind::Return { value } => self.visit_return(value),
            StmtKind::Break => self.visit_break(),
            StmtKind::While { condition, body } => self.visit_while(condition, body),
            StmtKind::ExprStmt(expr) => self.visit_expr_stmt(expr),
        };
        CSourceCode::from(format!("{}{}", "\t".repeat(self.indent), code))
    }

    fn visit_block(&mut self, statements: &'a [Stmt<Self::Type>]) -> Self::Output {
        self.begin_scope();
        let statements = visit_vec(
            statements,
            |stmt| format!("{}", self.visit_stmt(stmt)),
            "\n",
        );
        self.end_scope();
        format!("{{\n{statements}\n}}").into()
    }

    fn visit_let(
        &mut self,
        name: &Identifier,
        _: &Option<Typed>,
        initializer: &'a Expr<Self::Type>,
    ) -> CSourceCode {
        let name = CSourceCode::from(name);
        let var_type = CSourceCode::from(initializer.type_().as_ref());
        let initializer = self.visit_expr(initializer);
        format!("{var_type} {name} = {initializer};").into()
    }

    fn visit_return(&mut self, value: &'a Expr<Self::Type>) -> Self::Output {
        let value = self.visit_expr(value);
        format!("return {value};").into()
    }

    fn visit_break(&mut self) -> Self::Output {
        "break;".to_string().into()
    }

    fn visit_while(
        &mut self,
        condition: &'a Expr<Self::Type>,
        body: &'a Box<Stmt<Self::Type>>,
    ) -> Self::Output {
        let condition = self.visit_expr(condition);
        self.begin_scope();
        let body = self.visit_stmt(body);
        self.end_scope();
        format!("while ({condition}) {body}").into()
    }

    fn visit_expr_stmt(&mut self, expr: &'a Expr<Self::Type>) -> Self::Output {
        let expr = self.visit_expr(expr);
        format!("{expr};").into()
    }
}

impl<'a> DeclVisitor<'a> for CCodeGenerator {
    type Type = Typed;
    type Output = Option<CSourceCode>;

    fn visit_import(&mut self, _: &[Identifier], _: Source) -> Self::Output {
        None
    }

    fn visit_struct(&mut self, _: &DeclStruct, _: Source) -> Self::Output {
        None
    }

    fn visit_function(&mut self, function: &DeclFunc<Self::Type>, _: Source) -> Self::Output {
        if function.modifiers.is_extern {
            return None;
        }
        let name = CSourceCode::from(&function.name);
        let return_type = CSourceCode::from(&function.return_type);
        let params = visit_vec(
            &function.params,
            |x| format!("{}", CSourceCode::from(x)),
            ", ",
        );
        self.begin_scope();
        let body = visit_vec(&function.body, |x| format!("{}", self.visit_stmt(x)), "\n");
        self.end_scope();
        Some(CSourceCode::from(format!(
            "{return_type} {name}({params}) {{\n{body}\n}}"
        )))
    }

    fn visit_const(
        &mut self,
        name: &Identifier,
        type_: &Typed,
        initializer: &Expr<Self::Type>,
        _: Source,
    ) -> Self::Output {
        let name = CSourceCode::from(name);
        let var_type = CSourceCode::from(type_.as_ref());
        let initializer = self.visit_expr(initializer);
        Some(format!("{var_type} {name} = {initializer};").into())
    }
}
