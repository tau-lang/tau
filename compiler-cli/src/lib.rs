#![allow(clippy::needless_return)]
use crate::cli::{ArgsBuilder, HELP_MESSAGE};
use std::{env, fs, path::PathBuf, process::exit};
use tau_core::{
    context::Context,
    error::{DynResult, FileCache},
    generator::{Compiler, set_output},
    process_file,
};
#[cfg(feature = "c")]
use tau_generator_c::{CCodeGenerator, CHeaderGenerator};

pub mod cli;

pub fn main() {
    let args = ArgsBuilder::new()
        .parse(env::args().collect())
        .unwrap()
        .build();
    if args.input().is_empty() {
        println!("{}", HELP_MESSAGE)
    } else {
        let mut context = Context::new();
        // guaranteed to not be empty
        for filename in args.input() {
            if let Err(message) = compile_file(&mut context, filename, &args) {
                println!("{:?}", message);
                exit(2);
            }
        }
    }
}

pub fn compile_file(context: &mut Context, filename: &str, args: &cli::Args) -> DynResult {
    let filename = PathBuf::from(filename);
    if let Ok(content) = fs::read_to_string(&filename) {
        let cache = FileCache::new(content);
        let display = filename.display();
        let file_id = context.store(display.to_string(), cache.clone());
        match process_file(context, file_id, cache.as_ref()) {
            Ok((ast, _env)) => match args.target() {
                #[cfg(feature = "c")]
                cli::Target::C => {
                    let compiler = Compiler::new(&ast);
                    let header_output = set_output(filename.as_ref(), args.output(), "h");
                    compiler
                        .compile(CHeaderGenerator, &header_output)
                        .map_err(|e| Box::new(e) as Box<dyn std::fmt::Debug>)?;
                    let code_output = set_output(filename.as_ref(), args.output(), "c");
                    compiler
                        .compile(CCodeGenerator::new(), &code_output)
                        .map_err(|e| Box::new(e) as Box<dyn std::fmt::Debug>)
                }
                cli::Target::Cranelift => unimplemented!(),
            },
            Err(error) => context.handle(&error),
        }
    } else {
        println!("tau: cannot access {}: No such file", filename.display());
        exit(1);
    }
}
