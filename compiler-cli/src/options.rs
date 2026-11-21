//! Options that the CLI takes. These options are currently parsed with bpaf.
//! The documentation for the CLI is automatically generated inside the
//! `test-cli` package.

use crate::{code_gen::CodeGen, emit_type::EmitList};
use bpaf::*;
use std::{env, io, path::PathBuf};
use symbol_table::{GlobalSymbol, static_symbol};

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Options {
    /// Write output to compiler-chosen filename inside this directory
    #[bpaf(
        argument("DIR"),
        help("Write output to compiler-chosen filename in DIR")
    )]
    pub out_dir: Option<PathBuf>,
    /// Write output the filename base. For example, when using the C codegen
    /// and using the "main.xyz" output, it will write the files to "main.h" and
    /// "main.c" respectively.
    #[bpaf(
        short('o'),
        long("output"),
        argument("FILENAME"),
        help("Write output to FILENAME")
    )]
    pub output: Option<PathBuf>,
    /// The name of the crate. The compiler can not know the crate name that was
    /// specified in the build tool, so it needs to be passed here. If only a
    /// single file is generated for testing, not passing this option and
    /// keeping the name empty is generally fine.
    #[bpaf(
        argument("NAME"),
        help("Specify the NAME of the crate being built"),
        fallback(static_symbol!(""))
    )]
    pub crate_name: GlobalSymbol,
    /// The name of the module. The name can be guessed based on the input path,
    /// but the user may want to change this.
    #[bpaf(argument("NAME"), help("Specify the NAME of the module being built"))]
    pub module_name: Option<GlobalSymbol>,
    /// The code generator that generates the ir/asm/etc. code from the input.
    #[bpaf(
        short('C'),
        long("codegen"),
        argument("GEN"),
        help("Set the code generator"),
        fallback(CodeGen::default())
    )]
    pub code_gen: CodeGen,
    #[bpaf(
        long("emit"),
        help("Comma separeted list of types of output for the compiler to emit."),
        fallback(EmitList::default())
    )]
    pub emit: EmitList,
    #[bpaf(positional("PATH"), complete_shell(ShellComp::File { mask: Some("*.tau") }))]
    pub input: PathBuf,
}

impl Options {
    pub fn new(
        input: PathBuf,
        out_dir: Option<PathBuf>,
        output: Option<PathBuf>,
        crate_name: GlobalSymbol,
        module_name: Option<GlobalSymbol>,
        emit: EmitList,
        code_gen: CodeGen,
    ) -> Self {
        Self {
            input,
            out_dir,
            output,
            crate_name,
            module_name,
            emit,
            code_gen,
        }
    }

    /// Returns the input file.
    pub fn input(&self) -> &PathBuf {
        &self.input
    }

    /// Returns the output directory, defaults to the current directory.
    pub fn out_dir(&self) -> io::Result<PathBuf> {
        self.out_dir
            .as_ref()
            .map_or_else(env::current_dir, |c| Ok(c.to_path_buf()))
    }

    /// Returns the output file, defaults to the input file.
    pub fn output(&self) -> &PathBuf {
        self.output.as_ref().unwrap_or_else(|| self.input())
    }

    /// Returns the crate name, defaults to an empty crate name.
    pub fn crate_name(&self) -> GlobalSymbol {
        self.crate_name
    }

    /// Returns the module name, defaults to the input file prefix.
    pub fn module_name(&self) -> GlobalSymbol {
        self.module_name.unwrap_or_else(|| {
            self.input()
                .file_prefix()
                .map(|os_str| format!("{}", os_str.display()))
                .map_or_else(|| static_symbol!(""), GlobalSymbol::new)
        })
    }

    pub fn emit(&self) -> &EmitList {
        &self.emit
    }

    /// Returns the module name, defaults to QBE if enabled, otherwise to
    /// Cranelift.
    pub fn code_gen(&self) -> &CodeGen {
        &self.code_gen
    }
}

impl Default for Options {
    fn default() -> Self {
        Options {
            input: Default::default(),
            out_dir: Default::default(),
            output: Default::default(),
            crate_name: static_symbol!(""),
            module_name: Default::default(),
            emit: Default::default(),
            code_gen: Default::default(),
        }
    }
}
