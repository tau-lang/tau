use crate::LlvmCodeGen;
use inkwell::basic_block::BasicBlock;
use tau_core::{
    ast::{
        Identifier,
        expression::{Expr, ExprFolder},
        fold_vec,
        statement::{Stmt, StmtFolder},
    },
    error::DynResult,
    lexer::Source,
    type_::Typed,
};

impl<'a, 'ctx> LlvmCodeGen<'a, 'ctx> {
    fn fold_stmt_list(
        &mut self,
        statements: Vec<Stmt<Typed>>,
        label: BasicBlock<'ctx>,
    ) -> DynResult<()> {
        self.begin_scope();
        for statement in statements {
            self.fold_stmt(statement)?;
            if self.has_returned() {
                return Ok(());
            }
        }
        self.end_scope();
        self.builder.build_unconditional_branch(label)?;
        Ok(())
    }
}

impl<'a, 'ctx> StmtFolder for LlvmCodeGen<'a, 'ctx> {
    type Type = Typed;
    type Output = DynResult<()>;

    fn fold_block(&mut self, statements: Vec<Stmt<Self::Type>>, _: Source) -> Self::Output {
        self.begin_scope();
        fold_vec(statements, |statement| self.fold_stmt(statement))?;
        self.end_scope();
        Ok(())
    }

    fn fold_let(
        &mut self,
        name: Identifier,
        declared: Option<Typed>,
        initializer: Expr<Self::Type>,
        _: Source,
    ) -> Self::Output {
        let name = name.symbol();

        let initializer = self.fold_expr(initializer)?;
        let alloca =
            self.build_entry_block_alloca(name.as_str(), &declared.expect("type resolved"));
        self.builder.build_store(alloca, initializer)?;
        self.variables.insert(name, alloca);
        Ok(())
    }

    fn fold_return(&mut self, value: Option<Expr<Self::Type>>, _: Source) -> Self::Output {
        if let Some(value) = value {
            let value = self.fold_expr(value)?;
            self.builder.build_return(Some(&value))?;
        } else {
            self.builder.build_return(None)?;
        };
        self.mark_early_return();
        Ok(())
    }

    fn fold_break(&mut self, _: Source) -> Self::Output {
        self.mark_early_return();
        let label = self.loop_label();
        self.builder.build_unconditional_branch(*label)?;
        Ok(())
    }

    fn fold_if(
        &mut self,
        condition: Expr<Self::Type>,
        if_branch: Vec<Stmt<Self::Type>>,
        else_branch: Option<Vec<Stmt<Self::Type>>>,
        _: Source,
    ) -> Self::Output {
        let parent = self.fn_value();
        let condition = self.fold_expr(condition)?;
        let condition = self.as_condition(condition);

        // build branch
        let if_block = self.context.append_basic_block(parent, "if");
        let else_block = self.context.append_basic_block(parent, "else");
        let end_block = self.context.append_basic_block(parent, "end");

        self.builder
            .build_conditional_branch(condition, if_block, else_block)?;

        // if block
        self.builder.position_at_end(if_block);
        self.fold_stmt_list(if_branch, end_block)?;
        self.builder.get_insert_block().unwrap();

        // else block
        self.builder.position_at_end(else_block);
        if let Some(else_branch) = else_branch {
            self.fold_stmt_list(else_branch, end_block)?;
        } else {
            self.builder.build_unconditional_branch(end_block)?;
        }
        self.builder.get_insert_block().unwrap();

        // merge block
        self.builder.position_at_end(end_block);
        Ok(())
    }

    fn fold_while(
        &mut self,
        condition: Expr<Self::Type>,
        body: Vec<Stmt<Self::Type>>,
        _: Source,
    ) -> Self::Output {
        let parent = self.fn_value();

        let check_block = self.context.append_basic_block(parent, "check");
        let loop_block = self.context.append_basic_block(parent, "loop");
        let end_block = self.context.append_basic_block(parent, "end");

        // check if the condition holds
        self.builder.build_unconditional_branch(check_block)?;
        self.builder.position_at_end(check_block);
        let condition = self.fold_expr(condition)?;
        let condition = self.as_condition(condition);
        self.builder
            .build_conditional_branch(condition, loop_block, end_block)?;

        // execute the loop body
        self.begin_loop(end_block);
        self.builder.position_at_end(loop_block);
        self.fold_stmt_list(body, check_block)?;
        self.end_loop();

        self.builder.position_at_end(end_block);

        Ok(())
    }

    fn fold_expr_stmt(&mut self, expr: Expr<Self::Type>, _: Source) -> Self::Output {
        self.fold_expr(expr)?;
        Ok(())
    }
}
