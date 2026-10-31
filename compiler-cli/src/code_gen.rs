use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum CodeGen {
    #[cfg(feature = "c")]
    C,
    Cranelift,
}

impl Default for CodeGen {
    #[cfg(feature = "c")]
    fn default() -> Self {
        Self::C
    }

    #[cfg(not(feature = "c"))]
    fn default() -> Self {
        Self::Cranelift
    }
}

impl FromStr for CodeGen {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            #[cfg(feature = "c")]
            "c" => Ok(Self::C),
            "cranelift" => Ok(Self::Cranelift),
            _ => Err(format!("Could not convert {string} to target")),
        }
    }
}
