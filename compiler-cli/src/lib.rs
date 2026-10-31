#![allow(clippy::needless_return)]
use crate::code_gen::CodeGen;
use crate::options::{Options, options};
use std::{fs, process::exit, time::Instant};
use tau_core::{
    context::Context,
    error::{DynResult, FileCache},
    generator::{Compiler, output},
    process_file,
};
#[cfg(feature = "c")]
use tau_generator_c::{CCodeGenerator, CHeaderGenerator};

pub mod code_gen;
pub mod options;

pub fn main() {
    let args = options().run();
    let now = Instant::now();
    let mut context = Context::new();
    if let Err(message) = compile_file(&mut context, &args) {
        println!("{:?}", message);
        exit(2);
    }
    println!("Finished in {}s", now.elapsed().as_millis() as f64 / 1000.0);
}

pub fn compile_file(context: &mut Context, args: &Options) -> DynResult {
    let filename = args.input();
    if let Ok(content) = fs::read_to_string(&filename) {
        let cache = FileCache::new(content);
        let display = filename.display();
        let file_id = context.store(display.to_string(), cache.clone());
        match process_file(context, file_id, cache.as_ref()) {
            Ok((ast, _env)) => match args.code_gen() {
                #[cfg(feature = "c")]
                CodeGen::C => {
                    let folder = &args
                        .out_dir()
                        .map_err(|e| Box::new(e) as Box<dyn std::fmt::Debug>)?;
                    let compiler = Compiler::new(&ast);
                    let header_output = output(args.output(), folder, "h");
                    compiler
                        .compile(CHeaderGenerator, &header_output)
                        .map_err(|e| Box::new(e) as Box<dyn std::fmt::Debug>)?;
                    let code_output = output(args.output(), folder, "c");
                    compiler
                        .compile(CCodeGenerator::new(), &code_output)
                        .map_err(|e| Box::new(e) as Box<dyn std::fmt::Debug>)
                }
                CodeGen::Cranelift => unimplemented!(),
            },
            Err(error) => context.handle(&error),
        }
    } else {
        println!("tau: cannot access {}: No such file", filename.display());
        exit(1);
    }
}
