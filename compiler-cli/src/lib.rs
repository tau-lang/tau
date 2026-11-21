//! # Tau CLI
//!
//! This is the default CLI for the Tau programming language.
//!
//! ## Basic example
//!
//! ```rust
//! fn run() {
//!     tau_cli::main();
//! }
//! ```
//!
//! ## Optional features
//!
//! - `c` _(enabled by default)_ – Transpiler for Tau to C code
//! - `qbe` _(enabled by default)_ – Compiles the Tau ir to [QBE](https://c9x.me/compile/)
//! - `bright-color` – Enables bright coloring
//! - `dull-color` – Enables dull coloring

#![allow(clippy::needless_return)]
use crate::{
    code_gen::CodeGen,
    emit_type::EmitType,
    options::{Options, options},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::exit,
    time::Instant,
};
use tau_core::{
    ast::Unfold,
    context::Context,
    error::{DynResult, FileCache},
    process_file,
    translator::Translator,
};
#[cfg(feature = "c")]
use tau_generator_c::{CCodeGenerator, CHeaderGenerator};
use tau_generator_llvm::LlvmCodeGen;
#[cfg(feature = "qbe")]
use tau_generator_qbe::QbeEmitter;

pub mod code_gen;
pub mod emit_type;
pub mod options;

/// Parses the command line options and compiles the file.
/// This will handle any errors and measure the elapsed time.
pub fn main() {
    let args = options().run();
    let now = Instant::now();
    let mut context = Context::new();
    if let Err(message) = compile_file(&mut context, &args) {
        println!("{}", message);
        exit(2);
    }
    let elapsed = now.elapsed().as_millis() as f64 / 1000.0;
    if let Some(severity) = context.severity() {
        println!("{:?} after {}s", severity, elapsed);
        exit(1);
    } else {
        println!("Finished in {}s", elapsed);
    }
}

/// Compiles a single file from the given context and cli arguments.
///
/// # Examples
///
/// ```rust
/// use tau_cli::{options::options, compile_file};
/// use tau_core::context::Context;
/// use std::process::exit;
///
/// fn compiles() {
///     let args = options().run();
///     let mut context = Context::new();
///     if let Err(message) = compile_file(&mut context, &args) {
///         println!("{}", message);
///         exit(2);
///     }
/// }
/// ```
pub fn compile_file(context: &mut Context, args: &Options) -> DynResult<()> {
    let filename = args.input();
    let content = fs::read_to_string(filename)?;
    let cache = FileCache::new(content);
    let display = filename.display();
    let file_id = context.store(display.to_string(), cache.clone());
    match process_file(context, args.module_name(), file_id, cache.as_ref()) {
        Ok((ast, _env)) => match args.code_gen() {
            #[cfg(feature = "c")]
            CodeGen::C => {
                let folder = &args.out_dir()?;
                let header_output = output(folder, args.output(), "h");
                CHeaderGenerator.transpile(&ast, &header_output)?;
                let code_output = output(folder, args.output(), "c");
                CCodeGenerator::new().transpile(&ast, &code_output)
            }
            #[cfg(feature = "qbe")]
            CodeGen::Qbe => {
                let output = output(&args.out_dir()?, args.output(), "ssa");
                let mut emitter = QbeEmitter::new();
                Translator::new(&mut emitter).fold_ast(ast);
                Ok(fs::write(output, format!("{}", emitter.unfold()))?)
            }
            CodeGen::Llvm => {
                use inkwell::context::Context;

                let context = Context::create();
                let module = context.create_module(args.module_name().as_str());
                let mut emitter = LlvmCodeGen::new(&context, &module);
                emitter.fold_ast(ast)?;
                emitter.run_passes();

                let folder = &args.out_dir()?;
                for emit_type in args.emit() {
                    match emit_type {
                        EmitType::Obj => {
                            let object = output(folder, args.output(), "o");
                            emitter.write_object_file(&object)?;
                        }
                        EmitType::LlvmIr => {
                            let output = output(folder, args.output(), "ll");
                            module.print_to_file(output)?
                        }
                    }
                }
                Ok(())
            }
        },
        Err(error) => context.handle(&error),
    }
}

fn output(folder: &Path, path: &Path, extension: &str) -> PathBuf {
    let mut path_buf = PathBuf::from(folder);
    path_buf.push(path);
    path_buf.set_extension(extension);
    return path_buf;
}
