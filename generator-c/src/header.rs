use crate::{source_code::CSourceCode, visit_vec};
use std::{fs::File, io::prelude::*};
use tau_core::{
    ast::{
        Untyped,
        declaration::{Decl, DeclFunc, DeclStruct, DeclVisitor},
        expression::Expr,
        identifier::Identifier,
    },
    error,
    generator::Generator,
    lexer::Source,
};

pub struct CHeaderGenerator;

impl<'a> Generator<'a> for CHeaderGenerator {
    fn generate(
        &mut self,
        ast: &'a [Decl<Untyped>],
        output: &std::path::Path,
    ) -> error::Result<()> {
        let mut file = File::create(output).expect("error reading file");
        let mut path_buf = output.to_path_buf();
        path_buf.set_extension("");
        let module_name = path_buf
            .file_name()
            .expect("expect path has filename")
            .to_str()
            .expect("expect filename is UTF-8")
            .replace(".", "_")
            .replace("/", "$");
        let macro_name = format!("{}_H", module_name.to_uppercase());

        let declarations = visit_vec(ast, |x| format!("{}", self.visit_decl(x)), "\n\n");

        write!(
            file,
            "#ifndef {macro_name}\n#define {macro_name}\n\n{declarations}\n\n#endif\n"
        )
        .unwrap();
        Ok(())
    }
}

impl DeclVisitor<'_> for CHeaderGenerator {
    // TODO: switch to typed
    type Type = Untyped;
    type Output = CSourceCode;

    fn visit_import(&mut self, path: &[Identifier], _: &Source) -> CSourceCode {
        let path = path
            .iter()
            .map(|x| format!("{x}"))
            .collect::<Vec<String>>()
            .join("/");
        CSourceCode::from(format!("#include <{path}.hpp>"))
    }

    fn visit_struct(&mut self, structure: &DeclStruct, _: &Source) -> CSourceCode {
        let name = CSourceCode::from(&structure.name);
        let fields = structure
            .fields
            .iter()
            .map(|pair| {
                let (param_name, param_type) = pair;
                format!(
                    "\t{} {};",
                    CSourceCode::from(param_type),
                    CSourceCode::from(param_name)
                )
            })
            .collect::<Vec<String>>()
            .join("\n");
        format!("typedef struct {name} {{\n{fields}\n}} {name};").into()
    }

    fn visit_function(&mut self, function: &DeclFunc<Self::Type>, _: &Source) -> CSourceCode {
        let name = CSourceCode::from(&function.name);
        let extern_ = if function.modifiers.is_extern {
            "extern "
        } else {
            ""
        };
        let return_type = CSourceCode::from(function.return_type.borrow().as_ref());
        let params = visit_vec(
            &function.params,
            |x| format!("{}", CSourceCode::from(x)),
            ", ",
        );
        CSourceCode::from(format!("{extern_}{return_type} {name}({params});"))
    }

    fn visit_const(
        &mut self,
        var_name: &Identifier,
        var_type: &TypeCell,
        _: &Expr<Self::Type>,
        _: &Source,
    ) -> CSourceCode {
        let var_name = CSourceCode::from(var_name);
        let var_type = CSourceCode::from(var_type.borrow().as_ref());
        format!("extern {var_type} {var_name};").into()
    }
}
