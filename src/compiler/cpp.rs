use crate::{
    ast::{Decl, DeclVisitor, Expr, ExprVisitor, Identifier, Stmt, StmtVisitor},
    compiler::Generator,
    lexer::{Token, TokenType},
    typing::{TypeCell, TypeDef},
};
use std::{fs::File, io::Error, io::prelude::*, path::PathBuf, rc::Rc};

pub fn to_cpp_type<'a>(type_def: &TypeDef) -> String {
    match type_def {
        TypeDef::Struct { name, members: _ } => format!("{}*", name),
        TypeDef::Function {
            parameters: _,
            return_type: _,
        } => todo!(),
        TypeDef::Array(name) => format!("{}*", to_cpp_type(name)),
        TypeDef::Lazy(name) => panic!("lazy '{}' should have been deref", name),
        TypeDef::Number {
            name,
            size: _,
            float: _,
            signed: _,
        } => String::from(match *name {
            "i8" => "char",
            "i16" => "short",
            "i32" => "int",
            "i64" => "long",
            "u8" => "unsigned char",
            "u16" => "unsigned short",
            "u32" => "unsigned int",
            "u64" => "unsigned long",
            "f32" => "float",
            "f64" => "double",
            _ => panic!(),
        }),
        TypeDef::Native(name) => String::from(match *name {
            "str" => "char*",
            "bool" => "int",
            "void" => "void",
            _ => panic!(),
        }),
        TypeDef::Module {
            types: _,
            fields: _,
        }
        | TypeDef::Unknown => panic!(),
    }
}

pub fn patch_cpp_name(name: &str) -> &str {
    match name {
        "self" => "this",
        "this" => "$this",
        "new" => "$new",
        "yield" => "$yield",
        _ => name,
    }
}

pub struct CppHeaderGenerator;

impl<'a> CppHeaderGenerator {
    fn visit_method(
        &mut self,
        name: &Identifier,
        return_type: &TypeCell,
        params: &[(Identifier, TypeCell)],
    ) -> String {
        let mut builder = String::from("\n\t");
        builder.push_str(&to_cpp_type(return_type.borrow().as_ref()));
        builder.push_str(" ");
        builder.push_str(patch_cpp_name(name.get_name()));
        builder.push_str("(");
        let mut first = true;
        for (param_name, param_type) in params {
            if !first {
                builder.push_str(", ");
            } else {
                first = false;
            }
            builder.push_str(&to_cpp_type(param_type.borrow().as_ref()));
            builder.push_str(" ");
            builder.push_str(param_name.get_name());
        }
        builder.push_str(");\n");
        builder
    }
}

impl<'a> Generator<'a> for CppHeaderGenerator {
    fn generate(&mut self, declarations: &'a [Decl], output: &PathBuf) -> Result<(), Error> {
        let mut file = File::create(output)?;
        let mut builder = String::with_capacity(255);

        let file_name = output.file_name().expect("expect path has filename");
        let os_str = file_name.to_ascii_uppercase();
        let makro_name = os_str
            .to_str()
            .expect("expect filename is UTF-8")
            .replace(".", "_");
        let module_name = file_name
            .to_str()
            .expect("expect filename is UTF-8")
            .replace(".hpp", "")
            .replace("/", "_");

        for decl in declarations {
            builder.push_str(&self.visit_decl(decl));
        }
        write!(
            file,
            "#ifndef {}\n#define {} 1\n\nnamespace {} {{\n{}\n}}\n#endif",
            makro_name, makro_name, module_name, builder
        )
    }
}

impl DeclVisitor<'_, String> for CppHeaderGenerator {
    fn visit_import(&mut self, path: &[Identifier]) -> String {
        let mut builder = String::from("#include <");
        let mut first = true;
        for name in path {
            if first {
                first = false;
            } else {
                builder.push_str("/");
            }
            builder.push_str(name.get_name());
        }
        builder.push_str(".hpp>\n");
        builder
    }

    fn visit_struct(
        &mut self,
        name: &Identifier,
        fields: &[(Identifier, TypeCell)],
        methods: &[Rc<Decl>],
    ) -> String {
        let mut builder = String::from("\nclass ");
        builder.push_str(patch_cpp_name(name.get_name()));
        builder.push_str(" {\n  public:\n");
        for (field_name, field_type) in fields {
            builder.push_str("\t");
            builder.push_str(&to_cpp_type(field_type.borrow().as_ref()));
            builder.push_str(" ");
            builder.push_str(field_name.get_name());
            builder.push_str(";\n");
        }
        for method in methods {
            if let Decl::Function {
                name,
                return_type,
                params,
                body: _,
                is_extern: _,
            } = &**method
            {
                builder.push_str(&self.visit_method(name, return_type, params));
            } else {
                panic!("expected method");
            }
        }
        builder.push_str("};\n");
        builder
    }

    fn visit_function(
        &mut self,
        name: &Identifier,
        return_type: &TypeCell,
        params: &[(Identifier, TypeCell)],
        _: &[Stmt],
        is_extern: bool,
    ) -> String {
        let mut builder = String::from("\nextern ");
        if is_extern {
            builder.push_str("\"C\" ")
        }
        builder.push_str(&to_cpp_type(return_type.borrow().as_ref()));
        builder.push_str(" ");
        builder.push_str(patch_cpp_name(name.get_name()));
        builder.push_str("(");
        if params.len() > 0 {
            let (first_name, first_type) = params.get(0).unwrap();
            builder.push_str(&to_cpp_type(first_type.borrow().as_ref()));
            builder.push_str(" ");
            builder.push_str(first_name.get_name());
            for (param_name, param_type) in &params[1..] {
                builder.push_str(", ");
                builder.push_str(&to_cpp_type(param_type.borrow().as_ref()));
                builder.push_str(" ");
                builder.push_str(param_name.get_name());
            }
        }
        builder.push_str(");\n");
        builder
    }

    fn visit_const(&mut self, name: &Identifier, var_type: &TypeCell, _: &Expr) -> String {
        let mut builder = String::from("\nextern ");
        builder.push_str(&to_cpp_type(var_type.borrow().as_ref()));
        builder.push_str(" ");
        builder.push_str(name.get_name());
        builder.push_str(";\n");
        builder
    }
}

pub struct CppCodeGenerator {
    intendation: usize,
    main_type: Option<Rc<TypeDef>>,
}

impl<'a> CppCodeGenerator {
    pub fn new() -> CppCodeGenerator {
        CppCodeGenerator {
            intendation: 0,
            main_type: None,
        }
    }

    fn begin_scope(&mut self) {
        self.intendation += 1;
    }

    fn end_scope(&mut self) {
        self.intendation -= 1;
    }

    fn generate_main(return_type: Rc<TypeDef>, module_name: &str) -> String {
        let mut builder = String::from("int main() {\n");
        let return_type = to_cpp_type(return_type.as_ref());
        if return_type == "void" {
            builder.push_str(module_name);
            builder.push_str("::main();\n");
            builder.push_str("return 0;\n")
        } else if return_type == "int" {
            builder.push_str("return ");
            builder.push_str(module_name);
            builder.push_str("::main();\n")
        } else {
            panic!("unsupported return type")
        }
        builder.push_str("}");
        builder
    }

    fn visit_method(
        &mut self,
        self_type: &Identifier,
        name: &Identifier,
        return_type: &TypeCell,
        params: &[(Identifier, TypeCell)],
        body: &'a [Stmt],
        is_extern: bool,
    ) -> String {
        let mut builder = String::with_capacity(255);
        builder.push_str(&to_cpp_type(return_type.borrow().as_ref()));
        builder.push_str(" ");
        builder.push_str(patch_cpp_name(self_type.get_name()));
        builder.push_str("::");
        builder.push_str(name.get_name());
        builder.push_str("(");
        let mut first = true;
        for (param_name, param_type) in params {
            if !first {
                builder.push_str(", ");
            } else {
                first = false;
            }
            builder.push_str(&to_cpp_type(param_type.borrow().as_ref()));
            builder.push_str(" ");
            builder.push_str(param_name.get_name());
        }
        builder.push_str(")");
        if is_extern {
            builder.push_str(";\n")
        } else {
            self.begin_scope();
            builder.push_str(" {\n");
            for stmt in body {
                builder.push_str(&self.visit_stmt(stmt));
            }
            builder.push_str("}\n");
            self.end_scope();
        }
        builder
    }
}

impl<'a> Generator<'a> for CppCodeGenerator {
    fn generate(&mut self, ast: &'a [Decl], output: &PathBuf) -> Result<(), Error> {
        let mut builder = String::with_capacity(255);
        let mut file = File::create(output)?;
        let mut path_buf = output.clone();
        path_buf.set_extension("hpp");
        let header_file = path_buf
            .file_name()
            .expect("expect path has filename")
            .to_str()
            .expect("expect filename is UTF-8");
        let module_name = header_file.replace(".hpp", "").replace(".", "_");

        builder.push_str("namespace ");
        builder.push_str(&module_name);
        builder.push_str(" {\n");
        for decl in ast {
            builder.push_str(&self.visit_decl(decl));
        }
        builder.push_str("}\n");
        if let Some(main_type) = &self.main_type {
            builder.push_str(&Self::generate_main(main_type.clone(), &module_name));
        }

        write!(file, "#include \"{}\"\n{}", header_file, builder)
    }
}

impl<'a> ExprVisitor<'a, String> for CppCodeGenerator {
    fn visit_unary(&mut self, operator: &Token, right: &'a Rc<Expr>) -> String {
        match operator.get_type() {
            TokenType::Add => format!("+{}", self.visit_expr(right)),
            TokenType::Sub => format!("-{}", self.visit_expr(right)),
            TokenType::Not => format!("!{}", self.visit_expr(right)),
            _ => todo!(),
        }
    }

    fn visit_binary(
        &mut self,
        left: &'a Rc<Expr>,
        operator: &Token,
        right: &'a Rc<Expr>,
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

    fn visit_get(&mut self, left: &'a Rc<Expr>, right: &Identifier, lookup: &TypeCell) -> String {
        let left = self.visit_expr(left);
        let right = right.get_name();
        match lookup.borrow().as_ref() {
            TypeDef::Struct {
                name: _,
                members: _,
            } => format!("{}->{}", left, right),
            TypeDef::Module {
                types: _,
                fields: _,
            } => format!("{}::{}", left, right),
            error => panic!(
                "can only lookup structs and modules, not {:?} at {:?} with {:?}",
                error, left, right
            ),
        }
    }

    fn visit_index(&mut self, object: &'a Rc<Expr>, index: &'a Rc<Expr>, _: &TypeCell) -> String {
        format!("{}[{}]", self.visit_expr(object), self.visit_expr(index))
    }

    fn visit_call(&mut self, callee: &'a Rc<Expr>, arguments: &'a [Rc<Expr>]) -> String {
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
        array_type: &TypeCell,
        array_size: &Option<Rc<Expr>>,
        fields: &'a [Rc<Expr>],
    ) -> String {
        let mut builder = String::from("new ");
        builder.push_str(&to_cpp_type(array_type.borrow().as_ref()));
        builder.push_str("[");
        if let Some(array_size) = array_size {
            builder.push_str(&self.visit_expr(array_size));
        }
        builder.push_str("]");
        builder.push_str("{");
        for field in fields {
            builder.push_str(&self.visit_expr(field));
            builder.push_str(", ");
        }
        builder.push_str("}");
        builder
    }

    fn visit_create_struct(
        &mut self,
        struct_type: &TypeCell,
        fields: &'a [(Identifier, Rc<Expr>)],
    ) -> String {
        let mut builder = String::from("new ");
        if let TypeDef::Struct { name, members: _ } = struct_type.borrow().as_ref() {
            builder.push_str(name.get_name());
        } else {
            panic!("expected created type is a struct");
        }
        builder.push_str(" {");
        for (field_name, field_init) in fields {
            builder.push_str(".");
            builder.push_str(field_name.get_name());
            builder.push_str(" = ");
            builder.push_str(&self.visit_expr(field_init));
            builder.push_str(", ")
        }
        builder.push_str("}");
        builder
    }

    fn visit_if(
        &mut self,
        condition: &'a Rc<Expr>,
        if_branch: &'a Rc<Stmt>,
        else_branch: &'a Option<Rc<Stmt>>,
        expression_type: &TypeCell,
    ) -> String {
        if let TypeDef::Unknown = expression_type.borrow().as_ref() {
            let mut builder = String::from("if (");
            builder.push_str(&self.visit_expr(condition));
            builder.push_str(") ");
            builder.push_str(&self.visit_stmt(if_branch));
            if let Some(branch) = else_branch {
                builder.push_str("else ");
                builder.push_str(&self.visit_stmt(branch))
            }
            builder
        } else {
            let mut builder = String::from("(");
            builder.push_str(&self.visit_expr(condition));
            builder.push_str(" ? ");
            builder.push_str(&self.visit_stmt(if_branch));
            builder.push_str(" : ");
            let else_branch = else_branch
                .as_ref()
                .expect("expected else expression exists");
            builder.push_str(&self.visit_stmt(else_branch));
            builder.push_str(")");
            builder
        }
    }

    fn visit_literal(&mut self, value: &Token) -> String {
        match value.get_type() {
            TokenType::Bool(value) => value.to_string(),
            TokenType::Number(value) => value.to_string(),
            TokenType::String(content) => format!("\"{}\"", content),
            _ => todo!(),
        }
    }

    fn visit_variable(&mut self, name: &Identifier, _: &TypeCell) -> String {
        String::from(patch_cpp_name(name.get_name()))
    }
}

impl<'a> StmtVisitor<'a, String> for CppCodeGenerator {
    fn visit_stmt(&mut self, stmt: &'a Stmt) -> String {
        let intend = "\t".repeat(self.intendation);
        match stmt {
            Stmt::Block { statements } => self.visit_block(statements),
            Stmt::Let {
                name,
                var_type,
                initializer,
            } => format!("{}{}", intend, self.visit_let(name, var_type, initializer)),
            Stmt::Return { value } => format!("{}{}", intend, self.visit_return(value)),
            Stmt::Break => format!("{}{}", intend, self.visit_break()),
            Stmt::While { condition, body } => {
                format!("{}{}", intend, self.visit_while(condition, body))
            }
            Stmt::For {
                initializer,
                condition,
                increment,
                body,
            } => format!(
                "{}{}",
                intend,
                self.visit_for(initializer, condition, increment, body)
            ),
            Stmt::ExprStmt(expr) => format!("{}{}", intend, self.visit_expr_stmt(expr)),
        }
    }

    fn visit_block(&mut self, statements: &'a [Rc<Stmt>]) -> String {
        let mut builder = String::from("{\n");
        self.begin_scope();
        for stmt in statements {
            builder.push_str(&self.visit_stmt(stmt));
        }
        self.end_scope();
        builder.push_str(&"\t".repeat(self.intendation));
        builder.push_str("}");
        builder
    }

    fn visit_let(
        &mut self,
        name: &Identifier,
        var_type: &TypeCell,
        initializer: &'a Expr,
    ) -> String {
        let mut builder = String::with_capacity(64);
        builder.push_str(&to_cpp_type(var_type.borrow().as_ref()));
        builder.push_str(" ");
        builder.push_str(patch_cpp_name(name.get_name()));
        builder.push_str(" = ");
        builder.push_str(&self.visit_expr(initializer));
        builder.push_str(";\n");
        builder
    }

    fn visit_return(&mut self, value: &'a Expr) -> String {
        let mut builder = String::from("return ");
        builder.push_str(&self.visit_expr(value));
        builder.push_str(";\n");
        builder
    }

    fn visit_break(&mut self) -> String {
        String::from("break;\n")
    }

    fn visit_while(&mut self, condition: &'a Expr, body: &'a Rc<Stmt>) -> String {
        let mut builder = String::from("while (");
        builder.push_str(&self.visit_expr(condition));
        builder.push_str(") ");
        builder.push_str(&self.visit_stmt(body));
        builder.push_str("\n");
        builder
    }

    fn visit_for(
        &mut self,
        initializer: &'a Rc<Stmt>,
        condition: &'a Expr,
        increment: &'a Expr,
        body: &'a Rc<Stmt>,
    ) -> String {
        self.begin_scope();
        let mut builder = String::from("for (");
        builder.push_str(&self.visit_stmt(initializer));
        builder.push_str(&self.visit_expr(condition));
        builder.push_str(";");
        builder.push_str(&self.visit_expr(increment));
        builder.push_str(") ");
        builder.push_str(&self.visit_stmt(body));
        self.end_scope();
        builder
    }

    fn visit_expr_stmt(&mut self, expr: &'a Expr) -> String {
        let mut builder = self.visit_expr(expr);
        builder.push_str(";\n");
        builder
    }
}

impl<'a> DeclVisitor<'a, String> for CppCodeGenerator {
    fn visit_import(&mut self, _: &[Identifier]) -> String {
        String::new()
    }

    fn visit_struct(
        &mut self,
        name: &Identifier,
        _: &[(Identifier, TypeCell)],
        methods: &'a [Rc<Decl>],
    ) -> String {
        let mut builder = String::with_capacity(255);
        let self_type = name;
        for method in methods {
            if let Decl::Function {
                name,
                return_type,
                params,
                body,
                is_extern,
            } = method.as_ref()
            {
                builder.push_str(&self.visit_method(
                    self_type,
                    name,
                    return_type,
                    params,
                    body,
                    *is_extern,
                ))
            } else {
                panic!("expected method")
            }
        }
        builder
    }

    fn visit_function(
        &mut self,
        name: &Identifier,
        return_type: &TypeCell,
        params: &[(Identifier, TypeCell)],
        body: &'_ [Stmt],
        is_extern: bool,
    ) -> String {
        if name.get_name() == "main" {
            self.main_type = Some(return_type.borrow().clone())
        }
        if is_extern {
            return String::new();
        }

        let mut builder = String::with_capacity(255);
        builder.push_str(&to_cpp_type(return_type.borrow().as_ref()));
        builder.push_str(" ");
        builder.push_str(patch_cpp_name(name.get_name()));
        builder.push_str("(");
        if params.len() > 0 {
            let (first_name, first_type) = params.get(0).unwrap();
            builder.push_str(&to_cpp_type(first_type.borrow().as_ref()));
            builder.push_str(" ");
            builder.push_str(first_name.get_name());
            for (param_name, param_type) in &params[1..] {
                builder.push_str(", ");
                builder.push_str(&to_cpp_type(param_type.borrow().as_ref()));
                builder.push_str(" ");
                builder.push_str(param_name.get_name());
            }
        }
        builder.push_str(")");
        self.begin_scope();
        builder.push_str(" {\n");
        for stmt in body {
            builder.push_str(&self.visit_stmt(stmt));
        }
        builder.push_str("}\n");
        self.end_scope();
        builder
    }

    fn visit_const(
        &mut self,
        name: &Identifier,
        var_type: &TypeCell,
        initializer: &'_ Expr,
    ) -> String {
        let mut builder = String::from(&to_cpp_type(var_type.borrow().as_ref()));
        builder.push_str(" ");
        builder.push_str(name.get_name());
        builder.push_str(" = ");
        builder.push_str(&self.visit_expr(initializer));
        builder.push_str(";\n");
        builder
    }
}
