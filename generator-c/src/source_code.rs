use std::fmt::{self, Display, Formatter};
use tau_core::{
    ast::identifier::Identifier,
    type_::{Type, TypeVar, Typed},
};

#[derive(Default, PartialEq, Debug)]
pub struct CSourceCode(String);

impl CSourceCode {
    pub const fn as_str(&self) -> &str {
        &self.0.as_str()
    }
}

impl Display for CSourceCode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(formatter, "{}", self.0)
    }
}

impl From<&Type> for CSourceCode {
    fn from(type_def: &Type) -> Self {
        CSourceCode(match type_def {
            Type::Fn { .. } => todo!(),
            Type::Named { name, .. } => String::from(match name.as_str() {
                "str" => "char*",
                "bool" => "int",
                "void" => "void",
                "i8" => "int",
                "i16" => "short",
                "i32" => "int",
                "i64" => "long",
                "u8" => "unsigned int",
                "u16" => "unsigned short",
                "u32" => "unsigned int",
                "u64" => "unsigned long",
                "f32" => "float",
                "f64" => "double",
                other => other,
            }),
            Type::Var { type_ } => return CSourceCode::from(&*type_.borrow()),
            Type::Array { type_, size } => {
                format!("{}[{}]", CSourceCode::from(type_.as_ref()), size)
            }
            Type::Ptr { unchecked, type_ } => {
                let type_ = CSourceCode::from(type_.as_ref());
                let ptr = if *unchecked { '*' } else { '&' };
                format!("{}{}", type_, ptr)
            }
        })
    }
}

impl From<&Typed> for CSourceCode {
    fn from(typed: &Typed) -> Self {
        CSourceCode::from(&*typed.as_ref())
    }
}

impl From<&TypeVar> for CSourceCode {
    fn from(type_var: &TypeVar) -> Self {
        match type_var {
            TypeVar::Link { type_ } => CSourceCode::from(type_),
            _ => todo!(),
        }
    }
}

impl From<&Identifier> for CSourceCode {
    fn from(identifier: &Identifier) -> Self {
        CSourceCode(match identifier.name() {
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

impl From<&(Identifier, Typed)> for CSourceCode {
    fn from((name, type_): &(Identifier, Typed)) -> Self {
        let type_ = CSourceCode::from(type_.as_ref());
        // C specifies first the type, and then the name.
        // Therefore we need to swap the arguments here.
        CSourceCode(format!("{type_} {name}"))
    }
}

impl From<String> for CSourceCode {
    fn from(string: String) -> Self {
        CSourceCode(string)
    }
}
