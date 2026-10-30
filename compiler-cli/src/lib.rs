#![allow(clippy::needless_return)]
use crate::cli::{ArgsBuilder, HELP_MESSAGE};
use std::{env, fs, path::PathBuf, process::exit};
use tau_core::{
    context::Context,
    error::DynResult,
    generator::{Compiler, set_output},
    process_file,
};
use tau_generator_c::{CCodeGenerator, CHeaderGenerator};

mod cli;

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

fn compile_file(context: &mut Context, filename: &str, args: &cli::Args) -> DynResult {
    let filename = PathBuf::from(filename);
    if let Ok(content) = fs::read_to_string(&filename) {
        let display = filename.display();
        // TODO: remove clone!
        let file_id = context.store(display.to_string(), content.clone());
        match process_file(context, file_id, &content) {
            Ok((ast, _env)) => match args.target() {
                cli::Target::C => {
                    // FIXME: resolve types first
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

#[cfg(test)]
mod tests {
    use super::{Context, compile_file};
    use crate::cli::ArgsBuilder;

    macro_rules! assert_ok {
        ($result:expr) => {{
            let result = &$result;
            assert!(matches!(result, Ok(_)), "Expected Ok(_), got {:?}", result);
        }};
    }

    macro_rules! assert_none {
        ($result:expr) => {{
            let result = &$result;
            assert!(matches!(result, None), "Expected None, got {:?}", result);
        }};
    }

    #[test]
    fn pipeline_arrays() {
        let args = ArgsBuilder::new().build();
        let mut context = Context::new();
        let result = compile_file(&mut context, "../examples/arrays.tau", &args);
        assert_ok!(result);
        assert_none!(context.severity());
    }

    #[test]
    fn pipeline_heap() {
        let args = ArgsBuilder::new().build();
        let mut context = Context::new();
        let result = compile_file(&mut context, "../examples/heap.tau", &args);
        assert_ok!(result);
        assert_none!(context.severity());
    }

    #[test]
    fn pipeline_vec2() {
        let args = ArgsBuilder::new().build();
        let mut context = Context::new();
        let result = compile_file(&mut context, "../examples/vec2.tau", &args);
        assert_ok!(result);
        assert_none!(context.severity());
    }
}
