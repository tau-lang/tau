use crate::{
    ast::{
        declaration::{Decl, DeclVisitor},
        expression::{Expr, ExprVisitor},
        identifier::Identifier,
        statement::{Stmt, StmtVisitor},
    },
    compiler::Generator,
    lexer::{Token, TokenType},
    typing::{TypeCell, TypeDef},
};
use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    io::prelude::*,
    ops::Deref,
    rc::Rc,
};

#[derive(Default, PartialEq, Debug)]
pub struct CppSourceCode(String);

impl Display for CppSourceCode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(formatter, "{}", self.0)
    }
}

impl From<&TypeDef> for CppSourceCode {
    fn from(type_def: &TypeDef) -> Self {
        CppSourceCode(match type_def {
            TypeDef::Struct { name, members: _ } => format!("{}*", name),
            TypeDef::Function {
                parameters: _,
                return_type: _,
            } => todo!(),
            TypeDef::Array(name) => format!("{}*", CppSourceCode::from(name.as_ref())),
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
        })
    }
}

impl From<&TypeCell> for CppSourceCode {
    fn from(type_cell: &TypeCell) -> Self {
        CppSourceCode::from(type_cell.borrow().as_ref())
    }
}

impl From<&Identifier> for CppSourceCode {
    fn from(identifier: &Identifier) -> Self {
        CppSourceCode(match identifier.get_name() {
            "self" => "this".to_string(),
            name @ ("alignas" | "alignof" | "and" | "and_eq" | "asm" | "atomic_cancel"
            | "atomic_commit" | "atomic_noexcept" | "auto" | "bitand" | "bitor"
            | "case" | "catch" | "char8_t" | "char16_t" | "char32_t" | "class"
            | "compl" | "concept" | "consteval" | "constexpr" | "constinit"
            | "const_cast" | "continue" | "contract_assert" | "co_await" | "co_return"
            | "co_yield" | "decltype" | "default" | "delete" | "do" | "double"
            | "dynamic_cast" | "enum" | "explicit" | "export" | "friend" | "goto"
            | "inline" | "int" | "long" | "mutable" | "namespace" | "new" | "noexcept"
            | "not" | "not_eq" | "nullptr" | "operator" | "or" | "or_eq" | "private"
            | "protected" | "public" | "reflexpr" | "register" | "re") => format!("${}", name),
            name => name.to_string(),
        })
    }
}

impl From<&(Identifier, TypeCell)> for CppSourceCode {
    fn from((name, type_cell): &(Identifier, TypeCell)) -> Self {
        CppSourceCode(format!("{} {name}", CppSourceCode::from(type_cell)))
    }
}

impl From<String> for CppSourceCode {
    fn from(string: String) -> Self {
        CppSourceCode(string)
    }
}

fn visit_vec<T, F>(params: &[T], f: F, j: &str) -> CppSourceCode
where
    F: FnMut(&T) -> String,
{
    params.iter().map(f).collect::<Vec<String>>().join(j).into()
}

pub struct CppHeaderGenerator;

impl CppHeaderGenerator {
    fn visit_method(
        &mut self,
        name: &Identifier,
        return_type: &TypeCell,
        params: &[(Identifier, TypeCell)],
    ) -> CppSourceCode {
        let return_type = CppSourceCode::from(return_type.borrow().as_ref());
        let name = CppSourceCode::from(name);
        let params = visit_vec(params, |x| format!("{}", CppSourceCode::from(x)), ", ");
        CppSourceCode(format!("{return_type} {name}({params});"))
    }
}

impl<'a> Generator<'a> for CppHeaderGenerator {
    fn generate(&mut self, ast: &'a [Decl], output: &std::path::Path) -> crate::error::Result<()> {
        // TODO:
        let mut file = File::create(output).expect("error reading file");
        let mut path_buf = output.to_path_buf();
        path_buf.set_extension("");
        let module_name = path_buf
            .file_name()
            .expect("expect path has filename")
            .to_str()
            .expect("expect filename is UTF-8")
            .replace(".", "_")
            .replace("/", "$");
        let makro_name = module_name.to_uppercase();

        let declarations = visit_vec(ast, |x| format!("{}", self.visit_decl(x)), "\n");

        write!(
            file,
            "#ifndef {makro_name}\n#define {makro_name} 1\nnamespace {module_name} {{\n{declarations}\n}}\n#endif\n"
        ).unwrap();
        Ok(())
    }
}

impl DeclVisitor<'_> for CppHeaderGenerator {
    type Output = CppSourceCode;
    fn visit_import(&mut self, path: &[Identifier]) -> CppSourceCode {
        let path = path
            .iter()
            .map(|x| format!("{x}"))
            .collect::<Vec<String>>()
            .join("/");
        CppSourceCode(format!("#include <{path}.hpp>"))
    }

    fn visit_struct(
        &mut self,
        name: &Identifier,
        fields: &[(Identifier, TypeCell)],
        methods: &[Rc<Decl>],
    ) -> CppSourceCode {
        let name = CppSourceCode::from(name);
        let fields = visit_vec(fields, |x| format!("  {};", CppSourceCode::from(x)), "\n");
        let methods = visit_vec(
            methods,
            |decl| {
                if let Decl::Function {
                    name,
                    return_type,
                    params,
                    body: _,
                    is_extern: _,
                    is_io: _,
                } = decl.as_ref()
                {
                    format!("  {}", self.visit_method(name, return_type, params))
                } else {
                    panic!()
                }
            },
            "\n",
        );
        format!("class {name} {{\npublic:\n{fields}\n{methods}\n}};").into()
    }

    fn visit_function(
        &mut self,
        name: &Identifier,
        return_type: &TypeCell,
        params: &[(Identifier, TypeCell)],
        _: &[Stmt],
        is_extern: bool,
        _: bool,
    ) -> CppSourceCode {
        let name = CppSourceCode::from(name);
        let is_extern = if is_extern { "\"C\" " } else { "" };
        let return_type = CppSourceCode::from(return_type.borrow().as_ref());
        let params = visit_vec(params, |x| format!("{}", CppSourceCode::from(x)), ", ");
        CppSourceCode(format!("extern {is_extern}{return_type} {name}({params});"))
    }

    fn visit_const(
        &mut self,
        var_name: &Identifier,
        var_type: &TypeCell,
        _: &Expr,
    ) -> CppSourceCode {
        let var_name = CppSourceCode::from(var_name);
        let var_type = CppSourceCode::from(var_type.borrow().as_ref());
        format!("extern {var_type} {var_name};").into()
    }
}

pub struct CppCodeGenerator {
    main_type: Option<Rc<TypeDef>>,
}

impl<'a> CppCodeGenerator {
    pub fn new() -> CppCodeGenerator {
        CppCodeGenerator { main_type: None }
    }

    fn visit_main(&self, return_type: Rc<TypeDef>, module_name: &str) -> CppSourceCode {
        let return_type = CppSourceCode::from(return_type.as_ref());
        let body = match (return_type.0).deref() {
            "void" => format!("{module_name}::main();\nreturn 0;\n"),
            "int" => format!("return {module_name}::main();\n"),
            _ => panic!("unsupported return type"),
        };
        format!("int main() {{\n{body}}}\n").into()
    }

    fn visit_method(
        &mut self,
        struct_name: &Identifier,
        name: &Identifier,
        return_type: &TypeCell,
        params: &[(Identifier, TypeCell)],
        body: &'a [Stmt],
        _: bool,
    ) -> CppSourceCode {
        let struct_name = CppSourceCode::from(struct_name);
        let name = CppSourceCode::from(name);
        let return_type = CppSourceCode::from(return_type);
        let params = visit_vec(params, |x| format!("{}", CppSourceCode::from(x)), ", ");
        let body = visit_vec(body, |stmt| format!("{}", self.visit_stmt(stmt)), "\n");
        format!("{return_type} {struct_name}::{name}({params}) {{\n{body}\n}}").into()
    }
}

impl<'a> Generator<'a> for CppCodeGenerator {
    fn generate(&mut self, ast: &'a [Decl], output: &std::path::Path) -> crate::error::Result<()> {
        // TODO:
        let mut file = File::create(output).unwrap();
        let mut path_buf = output.to_path_buf();
        path_buf.set_extension("hpp");
        let header_file = path_buf
            .file_name()
            .expect("expect path has filename")
            .to_str()
            .expect("expect filename is UTF-8");
        let module_name = header_file.replace(".hpp", "").replace(".", "_");

        let declarations = visit_vec(ast, |decl| format!("{}", self.visit_decl(decl)), "\n");
        let main_function = if let Some(return_type) = &self.main_type {
            self.visit_main(return_type.clone(), &module_name)
        } else {
            CppSourceCode::default()
        };

        write!(
            file,
            "#include \"{header_file}\"\nnamespace {module_name} {{\n{declarations}\n}}\n{main_function}"
        ).unwrap();
        Ok(())
    }
}

impl<'a> ExprVisitor<'a> for CppCodeGenerator {
    type Output = CppSourceCode;

    fn visit_unary(&mut self, operator: &Token, right: &'a Rc<Expr>) -> CppSourceCode {
        match operator.get_type() {
            TokenType::Add => format!("+{}", self.visit_expr(right)),
            TokenType::Sub => format!("-{}", self.visit_expr(right)),
            TokenType::Not => format!("!{}", self.visit_expr(right)),
            _ => todo!(),
        }
        .into()
    }

    fn visit_binary(
        &mut self,
        left: &'a Rc<Expr>,
        operator: &Token,
        right: &'a Rc<Expr>,
    ) -> CppSourceCode {
        let mut render_op =
            |op: &str| format!("{} {op} {}", self.visit_expr(left), self.visit_expr(right));
        match operator.get_type() {
            TokenType::Add => render_op("+"),
            TokenType::Sub => render_op("-"),
            TokenType::Mul => render_op("*"),
            TokenType::Div => render_op("/"),
            TokenType::Eq => render_op("=="),
            TokenType::Neq => render_op("!="),
            TokenType::Low => render_op("<"),
            TokenType::Leq => render_op("<="),
            TokenType::Gre => render_op(">"),
            TokenType::Geq => render_op(">="),
            TokenType::And => render_op("&&"),
            TokenType::Or => render_op("||"),
            TokenType::Xor => render_op("|"),
            TokenType::Set => render_op("="),
            TokenType::SetAdd => render_op("+="),
            TokenType::SetSub => render_op("-="),
            TokenType::SetMul => render_op("*="),
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
    ) -> CppSourceCode {
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
        .into()
    }

    fn visit_index(
        &mut self,
        object: &'a Rc<Expr>,
        index: &'a Rc<Expr>,
        _: &TypeCell,
    ) -> CppSourceCode {
        let object = self.visit_expr(object);
        let index = self.visit_expr(index);
        format!("{object}[{index}]").into()
    }

    fn visit_call(&mut self, callee: &'a Rc<Expr>, arguments: &'a [Rc<Expr>]) -> CppSourceCode {
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
    ) -> CppSourceCode {
        let array_type = CppSourceCode::from(array_type);
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
    ) -> CppSourceCode {
        let struct_type = struct_type.borrow();
        let struct_type = if let TypeDef::Struct { name, members: _ } = struct_type.as_ref() {
            name.get_name()
        } else {
            unreachable!()
        };
        let fields = visit_vec(
            fields,
            |(field_name, field_init)| {
                format!(
                    ".{} = {}",
                    CppSourceCode::from(field_name),
                    self.visit_expr(field_init)
                )
            },
            ", ",
        );
        format!("new {struct_type}{{{fields}}}").into()
    }

    fn visit_if(
        &mut self,
        condition: &'a Rc<Expr>,
        if_branch: &'a Rc<Stmt>,
        else_branch: &'a Option<Rc<Stmt>>,
        expression_type: &TypeCell,
    ) -> CppSourceCode {
        let condition = self.visit_expr(condition);
        let if_branch = self.visit_stmt(if_branch);
        if let TypeDef::Unknown = expression_type.borrow().as_ref() {
            let else_branch = else_branch
                .as_ref()
                .map(|expr| format!(" else {}", self.visit_stmt(expr.as_ref())))
                .unwrap_or_default();
            format!("if ({condition}) {if_branch}{else_branch}").into()
        } else {
            let else_branch = self.visit_stmt(
                else_branch
                    .as_ref()
                    .expect("expected else expression exists"),
            );
            format!("{condition} ? {if_branch} : {else_branch}").into()
        }
    }

    fn visit_literal(&mut self, value: &Token) -> CppSourceCode {
        match value.get_type() {
            TokenType::Bool(value) => value.to_string(),
            TokenType::Number(value) => value.to_string(),
            TokenType::String(content) => format!("\"{}\"", content),
            _ => todo!(),
        }
        .into()
    }

    fn visit_variable(&mut self, name: &Identifier, _: &TypeCell) -> CppSourceCode {
        name.into()
    }
}

impl<'a> StmtVisitor<'a> for CppCodeGenerator {
    type Output = CppSourceCode;

    fn visit_block(&mut self, statements: &'a [Rc<Stmt>]) -> Self::Output {
        let statements = visit_vec(
            statements,
            |stmt| format!("{}", self.visit_stmt(stmt)),
            "\n",
        );
        format!("{{\n{statements}\n}}").into()
    }

    fn visit_let(
        &mut self,
        name: &Identifier,
        var_type: &TypeCell,
        initializer: &'a Expr,
    ) -> CppSourceCode {
        let name = CppSourceCode::from(name);
        let var_type = CppSourceCode::from(var_type);
        let initializer = self.visit_expr(initializer);
        format!("{var_type} {name} = {initializer};").into()
    }

    fn visit_return(&mut self, value: &'a Expr) -> Self::Output {
        let value = self.visit_expr(value);
        format!("return {value};").into()
    }

    fn visit_break(&mut self) -> Self::Output {
        "break;".to_string().into()
    }

    fn visit_while(&mut self, condition: &'a Expr, body: &'a Rc<Stmt>) -> Self::Output {
        let condition = self.visit_expr(condition);
        let body = self.visit_stmt(body);
        format!("while ({condition}) {body}").into()
    }

    fn visit_for(
        &mut self,
        initializer: &'a Rc<Stmt>,
        condition: &'a Expr,
        increment: &'a Expr,
        body: &'a Rc<Stmt>,
    ) -> Self::Output {
        let initializer = self.visit_stmt(initializer);
        let condition = self.visit_expr(condition);
        let increment = self.visit_expr(increment);
        let body = self.visit_stmt(body);
        format!("for ({initializer}; {condition}; {increment}) {body}").into()
    }

    fn visit_expr_stmt(&mut self, expr: &'a Expr) -> Self::Output {
        let expr = self.visit_expr(expr);
        format!("{expr};").into()
    }
}

impl<'a> DeclVisitor<'a> for CppCodeGenerator {
    type Output = CppSourceCode;

    fn visit_import(&mut self, _: &[Identifier]) -> Self::Output {
        CppSourceCode::default()
    }

    fn visit_struct(
        &mut self,
        name: &Identifier,
        _: &[(Identifier, TypeCell)],
        methods: &'a [Rc<Decl>],
    ) -> CppSourceCode {
        let struct_type = name;
        visit_vec(
            methods,
            |method| {
                if let Decl::Function {
                    name,
                    return_type,
                    params,
                    body,
                    is_extern,
                    is_io: _,
                } = method.as_ref()
                {
                    format!("{}", self.visit_method(
                    struct_type,
                    name,
                    return_type,
                    params,
                    body,
                    *is_extern,
                ))
                } else {
                    panic!("expected method")
                }
            },
            "\n",
        )
    }

    fn visit_function(
        &mut self,
        name: &Identifier,
        return_type: &TypeCell,
        params: &[(Identifier, TypeCell)],
        body: &'_ [Stmt],
        is_extern: bool,
        _: bool,
    ) -> Self::Output {
        if name.get_name() == "main" {
            self.main_type = Some(return_type.borrow().clone())
        }
        if is_extern {
            return CppSourceCode::default();
        }
        let name = CppSourceCode::from(name);
        let return_type = CppSourceCode::from(return_type);
        let params = visit_vec(params, |x| format!("{}", CppSourceCode::from(x)), ", ");
        let body = visit_vec(body, |x| format!("{}", self.visit_stmt(x)), "\n");
        format!("{return_type} {name} ({params}) {{\n{body}\n}}").into()
    }

    fn visit_const(
        &mut self,
        name: &Identifier,
        var_type: &TypeCell,
        initializer: &'_ Expr,
    ) -> Self::Output {
        let name = CppSourceCode::from(name);
        let var_type = CppSourceCode::from(var_type);
        let initializer = self.visit_expr(initializer);
        format!("{var_type} {name} = {initializer};").into()
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::Source;

    use super::*;

    #[test]
    fn patch_cpp_name() {
        let expected = vec![
            "this",
            "$alignas",
            "$alignof",
            "$and",
            "$and_eq",
            "$asm",
            "$atomic_cancel",
            "$atomic_commit",
            "$atomic_noexcept",
            "$auto",
            "$bitand",
            "$bitor",
            "$case",
            "$catch",
            "$char8_t",
            "$char8_t",
            "$char16_t",
            "$char32_t",
            "$class",
            "$compl",
            "$concept",
            "$consteval",
            "$constexpr",
            "$constinit",
            "$const_cast",
            "$continue",
            "$contract_assert",
            "$co_await",
            "$co_return",
            "$co_yield",
            "$decltype",
            "$default",
            "$delete",
            "$do",
            "$double",
            "$dynamic_cast",
            "$enum",
            "$explicit",
            "$export",
            "$friend",
            "$goto",
            "$inline",
            "$int",
            "$long",
            "$mutable",
            "$namespace",
            "$new",
            "$noexcept",
            "$not",
            "$not_eq",
            "$nullptr",
            "$operator",
            "$or",
            "$or_eq",
            "$private",
            "$protected",
            "$public",
            "$reflexpr",
            "$register",
            "$re",
        ]
        .iter()
        .map(|name| CppSourceCode::from(name.to_string()))
        .collect::<Vec<CppSourceCode>>();
        let names = vec![
            "self",
            "alignas",
            "alignof",
            "and",
            "and_eq",
            "asm",
            "atomic_cancel",
            "atomic_commit",
            "atomic_noexcept",
            "auto",
            "bitand",
            "bitor",
            "case",
            "catch",
            "char8_t",
            "char8_t",
            "char16_t",
            "char32_t",
            "class",
            "compl",
            "concept",
            "consteval",
            "constexpr",
            "constinit",
            "const_cast",
            "continue",
            "contract_assert",
            "co_await",
            "co_return",
            "co_yield",
            "decltype",
            "default",
            "delete",
            "do",
            "double",
            "dynamic_cast",
            "enum",
            "explicit",
            "export",
            "friend",
            "goto",
            "inline",
            "int",
            "long",
            "mutable",
            "namespace",
            "new",
            "noexcept",
            "not",
            "not_eq",
            "nullptr",
            "operator",
            "or",
            "or_eq",
            "private",
            "protected",
            "public",
            "reflexpr",
            "register",
            "re",
        ]
        .into_iter()
        .map(|name| {
            Identifier::from(Token::new(
                TokenType::Identifier(name.to_string()),
                Source::default(),
            ))
        });
        let values = names
            .map(|name| CppSourceCode::from(&name))
            .collect::<Vec<CppSourceCode>>();
        assert_eq!(values, expected);
    }
}
