use std::fmt::{self, Display, Formatter};
use tau_core::{ast::identifier::Identifier, type_::Type};

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
            Type::Struct(structure) => {
                let mut path = String::new();
                path.push_str(structure.name[0].name());
                for item in &structure.name[1..] {
                    path.push_str("::");
                    path.push_str(item.name());
                }
                path.into()
            }
            Type::Function(_) => todo!(),
            Type::Array(name) => format!("{}*", CSourceCode::from(name.as_ref())),
            Type::Path(name) => panic!("path '{:?}' should have been deref", name),
            Type::Number(number) => {
                if number.float {
                    match number.size {
                        32 => "float",
                        64 => "double",
                        _ => unreachable!(),
                    }
                    .into()
                } else {
                    let int = match number.size {
                        8 => "char",
                        16 => "short",
                        32 => "int",
                        64 => "long",
                        _ => unreachable!(),
                    };
                    if number.signed {
                        int.into()
                    } else {
                        format!("unsigned {int}")
                    }
                }
            }
            Type::Native(name) => String::from(match *name {
                "str" => "char*",
                "bool" => "int",
                "void" => "void",
                _ => unreachable!(),
            }),
            Type::RawPointer(ptr) => format!("{}*", CSourceCode::from(ptr.as_ref())),
            other => panic!("unhandled type: {}", other),
        })
    }
}

impl From<&TypeCell> for CSourceCode {
    fn from(type_cell: &TypeCell) -> Self {
        CSourceCode::from(type_cell.borrow().as_ref())
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

impl From<&(Identifier, TypeCell)> for CSourceCode {
    fn from((name, type_cell): &(Identifier, TypeCell)) -> Self {
        CSourceCode(format!("{} {name}", CSourceCode::from(type_cell)))
    }
}

impl From<String> for CSourceCode {
    fn from(string: String) -> Self {
        CSourceCode(string)
    }
}
