use crate::LlvmCodeGen;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use tau_core::{
    ast::{
        Identifier,
        declaration::{DeclFolder, DeclVisitor, FuncDecl, StructDecl},
        expression::{Expr, ExprFolder},
        fold_vec,
        statement::StmtFolder,
    },
    error::DynResult,
    lexer::Source,
    type_::Typed,
};

impl<'a, 'b, 'ctx> DeclVisitor<'a> for LlvmCodeGen<'b, 'ctx> {
    type Type = Typed;
    type Output = DynResult<()>;

    fn visit_import(&mut self, _path: &'a [Identifier], _: Source) -> Self::Output {
        Ok(())
    }

    fn visit_struct(&mut self, struct_: &'a StructDecl, _: Source) -> Self::Output {
        self.context
            .opaque_struct_type(struct_.name.symbol().as_str());
        Ok(())
    }

    fn visit_function(&mut self, function: &'a FuncDecl<Self::Type>, _: Source) -> Self::Output {
        let arguments = function
            .params
            .iter()
            .map(|(_, type_)| self.fold_type(type_).into())
            .collect::<Vec<BasicMetadataTypeEnum>>();
        // Tau does not yet support var args.
        let is_var_args = false;
        let fn_type = if function.return_type.is_void() {
            self.context.void_type().fn_type(&arguments, is_var_args)
        } else {
            let return_type = self.fold_type(&function.return_type);
            return_type.fn_type(&arguments, is_var_args)
        };
        let fn_value = self
            .module
            .add_function(function.name.symbol().as_str(), fn_type, None);

        // set argument names
        for (arg, (name, _)) in fn_value.get_param_iter().zip(&function.params) {
            arg.set_name(name.symbol().as_str());
        }

        Ok(())
    }

    fn visit_const(
        &mut self,
        _name: &'a Identifier,
        _type_: &'a Typed,
        _initializer: &'a Expr<Self::Type>,
        _: Source,
    ) -> Self::Output {
        Ok(())
    }
}

impl<'a, 'ctx> DeclFolder for LlvmCodeGen<'a, 'ctx> {
    type Type = Typed;
    type Output = DynResult<()>;

    fn fold_import(&mut self, _path: Vec<Identifier>, _: Source) -> Self::Output {
        todo!()
    }

    fn fold_struct(&mut self, struct_: StructDecl, _: Source) -> Self::Output {
        let name = struct_.name.symbol();

        let fields = struct_
            .fields
            .iter()
            .map(|(_, type_)| self.fold_type(type_))
            .collect::<Vec<BasicTypeEnum>>();

        let indices = struct_
            .fields
            .iter()
            .enumerate()
            .map(|(index, (name, _))| (name.symbol(), index as u32))
            .collect();

        let struct_type = self
            .context
            .get_struct_type(name.as_str())
            .expect("opaque type exists");
        struct_type.set_body(&fields, false);

        self.structs.insert(name, indices);
        Ok(())
    }

    fn fold_function(&mut self, func: FuncDecl<Self::Type>, _: Source) -> Self::Output {
        if func.modifiers.is_extern {
            return Ok(());
        }

        let function = self
            .get_function(func.name.name())
            .expect("prototype exists");
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);
        self.fn_value = Some(function);

        for (arg, (name, type_)) in function.get_param_iter().zip(func.params) {
            let alloca = self.build_entry_block_alloca(name.symbol().as_str(), &type_);
            self.builder.build_store(alloca, arg)?;
            self.variables.insert(name.symbol(), alloca);
        }

        self.begin_scope();
        fold_vec(func.body, |statement| self.fold_stmt(statement))?;
        // Return in void functions is implicit
        if !self.has_returned() && func.return_type.is_void() {
            self.builder.build_return(None)?;
        }
        self.end_scope();

        if function.verify(true) {
            Ok(())
        } else {
            function.print_to_stderr();
            unsafe {
                function.delete();
            }

            Err("Invalid generated function.".into())
        }
    }

    fn fold_const(
        &mut self,
        _name: Identifier,
        type_: Typed,
        initializer: Expr<Self::Type>,
        _: Source,
    ) -> Self::Output {
        let _type_ = self.fold_type(&type_);
        let _initializer = self.fold_expr(initializer)?;
        todo!()
    }
}
