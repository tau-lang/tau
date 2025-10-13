use crate::compiler::Generator;
use cranelift::prelude::*;
use cranelift_object::{ObjectBuilder, ObjectModule};

pub struct CraneliftGenerator;

const TARGET_TRIPLE: &str = "x86_64-unknown-linux";
const ENTRYPOINT_FUNCTION_SYMBOL: &str = "main";

impl Generator for CraneliftGenerator {
    fn generate(
        &mut self,
        ast: &[crate::ast::Decl],
        output: &mut std::fs::File,
    ) -> Result<(), std::io::Error> {
        let isa = {
            let mut builder = settings::builder();
            // disable optimizations so dissassembly will more directly correlated to our Cranelift usage
            builder.set("opt_level", "none").unwrap();
            builder.enable("is_pic").unwrap();
            let flags = settings::Flags::new(builder);
            isa::lookup_by_name(TARGET_TRIPLE)
                .unwrap()
                .finish(flags)
                .unwrap()
        };
        let mut module = {
            // TODO:
            let translation_unit_name = b"output_a_binary";
            let libcall_names = cranelift_module::default_libcall_names();
            let builder =
                ObjectBuilder::new(isa.clone(), translation_unit_name, libcall_names).unwrap();
            ObjectModule::new(builder)
        };
        for decl in ast {
            match decl {
                crate::ast::Decl::Import(token) => todo!(),
                crate::ast::Decl::Struct {
                    name,
                    fields,
                    methods,
                } => todo!(),
                crate::ast::Decl::Function {
                    name,
                    return_type,
                    params,
                    body,
                } => todo!(),
                crate::ast::Decl::Const {
                    name,
                    var_type,
                    initializer,
                } => todo!(),
            }
        }
        todo!()
    }
}
