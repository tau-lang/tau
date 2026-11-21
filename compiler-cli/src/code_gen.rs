use std::str::FromStr;

/// Codegen options that the CLI supports for generating code.
/// The generator can be changed by passing the `-C` flag.
#[derive(Debug, Clone, Default)]
pub enum CodeGen {
    /// Transpiler for generating C code
    #[cfg(feature = "c")]
    C,
    #[default]
    /// Llvm
    Llvm,
    /// QBE <https://c9x.me/compile/>
    #[cfg(feature = "qbe")]
    Qbe,
}

impl FromStr for CodeGen {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string.to_lowercase().as_str() {
            #[cfg(feature = "c")]
            "c" => Ok(Self::C),
            "llvm" => Ok(Self::Llvm),
            #[cfg(feature = "qbe")]
            "qbe" => Ok(Self::Qbe),
            option => Err(format!(
                "`{option}` doesn't exist, did you mean llvm, c, or qbe?"
            )),
        }
    }
}
