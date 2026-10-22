use crate::{source_code::CSourceCode, visit_vec};
use std::{fs::File, io::prelude::*, rc::Rc};
use tau_core::{
    ast::{
        Typed, Untyped, UntypedDecl,
        declaration::{DeclFunc, DeclStruct, DeclVisitor},
        expression::{Expr, UntypedExprVisitor},
        identifier::Identifier,
        statement::{Stmt, StmtKind, StmtVisitor},
    },
    error,
    generator::Generator,
    lexer::{Source, Token, TokenType},
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
    fn generate(&mut self, ast: &'a [UntypedDecl], output: &std::path::Path) -> error::Result<()> {
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

impl<'a> UntypedExprVisitor<'a> for CCodeGenerator {
    type Output = CSourceCode;

    fn visit_unary(&mut self, operator: &Token, right: &'a Rc<Expr>) -> CSourceCode {
        match operator.token_type() {
            TokenType::Add => format!("+{}", self.visit_expr(right)),
            TokenType::Sub => format!("-{}", self.visit_expr(right)),
            TokenType::Not => format!("!{}", self.visit_expr(right)),
            TokenType::Star => format!("(*{})", self.visit_expr(right)),
            TokenType::Ref => format!("(&{})", self.visit_expr(right)),
            _ => todo!(),
        }
        .into()
    }

    fn visit_binary(
        &mut self,
        left: &'a Rc<Expr>,
        operator: &Token,
        right: &'a Rc<Expr>,
    ) -> CSourceCode {
        let mut render_op =
            |op: &str| format!("{} {op} {}", self.visit_expr(left), self.visit_expr(right));
        match operator.token_type() {
            TokenType::Add => render_op("+"),
            TokenType::Sub => render_op("-"),
            TokenType::Star => render_op("*"),
            TokenType::Div => render_op("/"),
            TokenType::Eq => render_op("=="),
            TokenType::Neq => render_op("!="),
            TokenType::Low => render_op("<"),
            TokenType::Leq => render_op("<="),
            TokenType::Gre => render_op(">"),
            TokenType::Geq => render_op(">="),
            TokenType::And => render_op("&&"),
            TokenType::Or => render_op("||"),
            TokenType::Vbar => render_op("|"),
            TokenType::Set => render_op("="),
            TokenType::SetAdd => render_op("+="),
            TokenType::SetSub => render_op("-="),
            TokenType::SetStar => render_op("*="),
            TokenType::SetDiv => render_op("/="),
            _ => todo!("{:?}", operator),
        }
        .into()
    }

    fn visit_get(
        &mut self,
        left: &'a Rc<Expr>,
        right: &Identifier,
        lookup: &TypeCell,
    ) -> CSourceCode {
        let left = self.visit_expr(left);
        let right = right.name();
        match lookup.borrow().as_ref() {
            TypeDef::Struct(_) => format!("{}.{}", left, right),
            error => panic!(
                "can only lookup structs and modules, not {:?} at {:?} with {:?}",
                error, left, right
            ),
        }
        .into()
    }

    fn visit_index(
        &mut self,
        object: &'a Rc<Expr>,
        index: &'a Rc<Expr>,
        _: &TypeCell,
    ) -> CSourceCode {
        let object = self.visit_expr(object);
        let index = self.visit_expr(index);
        format!("{object}[{index}]").into()
    }

    fn visit_call(&mut self, callee: &'a Rc<Expr>, arguments: &'a [Rc<Expr>]) -> CSourceCode {
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
        array_type: &TypeCell,
        array_size: &Option<Rc<Expr>>,
        fields: &'a [Rc<Expr>],
    ) -> CSourceCode {
        let array_type = CSourceCode::from(array_type);
        let array_size = array_size
            .as_ref()
            .map(|expr| self.visit_expr(expr.as_ref()))
            .unwrap_or_default();
        let fields = visit_vec(fields, |x| format!("{}", self.visit_expr(x)), ", ");
        format!("new {array_type}[{array_size}]{{{fields}}}").into()
    }

    fn visit_create_struct(
        &mut self,
        struct_type: &TypeCell,
        fields: &'a [(Identifier, Rc<Expr>)],
    ) -> CSourceCode {
        let struct_type: CSourceCode = struct_type.into();
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
        format!("({struct_type}) {{{fields}}}").into()
    }

    fn visit_if(
        &mut self,
        condition: &'a Rc<Expr>,
        if_branch: &'a Rc<Stmt>,
        else_branch: &'a Option<Rc<Stmt>>,
    ) -> CSourceCode {
        let condition = self.visit_expr(condition);
        if let TypeDef::Unknown = expression_type.borrow().as_ref() {
            let if_branch = self.visit_stmt(if_branch);
            if let Some(else_branch) = else_branch {
                let else_branch = self.visit_stmt(else_branch);
                format!("if ({condition}) {{ {if_branch} }} else {{ {else_branch} }}").into()
            } else {
                format!("if ({condition}) {{ {if_branch} }}").into()
            }
        } else {
            let stmt2expr = |stmt: &'a Stmt| match stmt.kind() {
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

    fn visit_literal(&mut self, value: &Token) -> CSourceCode {
        match value.token_type() {
            TokenType::Bool(value) => value.to_string(),
            TokenType::Number(value) => value.to_string(),
            TokenType::String(content) => format!("\"{}\"", content),
            _ => todo!(),
        }
        .into()
    }

    fn visit_variable(&mut self, name: &Identifier, _: &TypeCell) -> CSourceCode {
        name.into()
    }
}

impl<'a> StmtVisitor<'a> for CCodeGenerator {
    type Type = Untyped;
    type Output = CSourceCode;

    fn visit_stmt(&mut self, stmt: &'a Stmt<Self::Type>) -> Self::Output {
        let code = match stmt.kind() {
            StmtKind::Block { statements } => self.visit_block(statements),
            StmtKind::Let {
                name,
                var_type,
                initializer,
            } => self.visit_let(name, var_type, initializer),
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
        type_: &Typed,
        initializer: &'a Expr<Self::Type>,
    ) -> CSourceCode {
        let name = CSourceCode::from(name);
        let var_type = CSourceCode::from(type_);
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
        body: &'a Rc<Stmt<Self::Type>>,
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
    type Type = Untyped;
    type Output = Option<CSourceCode>;

    fn visit_import(&mut self, _: &[Identifier], _: &Source) -> Self::Output {
        None
    }

    fn visit_struct(&mut self, _: &DeclStruct, _: &Source) -> Self::Output {
        None
    }

    fn visit_function(&mut self, function: &DeclFunc<Self::Type>, _: &Source) -> Self::Output {
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
        _: &Source,
    ) -> Self::Output {
        let name = CSourceCode::from(name);
        let var_type = CSourceCode::from(type_);
        let initializer = self.visit_expr(initializer);
        Some(format!("{var_type} {name} = {initializer};").into())
    }
}
