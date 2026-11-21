use std::{slice, str::FromStr};

/// Types of files the compiler emits.
#[derive(Debug, Clone, Default)]
pub enum EmitType {
    LlvmIr,
    #[default]
    Obj,
}

impl FromStr for EmitType {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string.to_lowercase().as_str() {
            "llvm-ir" => Ok(Self::LlvmIr),
            "obj" => Ok(Self::Obj),
            option => Err(format!(
                "`{option}` doesn't exist, did you mean llvm-ir or obj?"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmitList(Vec<EmitType>);

impl FromStr for EmitList {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let mut items = Vec::new();
        for item in string.split(",") {
            items.push(EmitType::from_str(item.trim())?);
        }
        Ok(EmitList(items))
    }
}

impl Default for EmitList {
    fn default() -> Self {
        Self(vec![EmitType::default()])
    }
}

impl<'a> IntoIterator for &'a EmitList {
    type Item = &'a EmitType;
    type IntoIter = slice::Iter<'a, EmitType>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
