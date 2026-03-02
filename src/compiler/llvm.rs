use crate::{
    ast::{
        expression::{Expr, ExprVisitor},
        identifier::Identifier,
        statement::{Stmt, StmtVisitor},
    },
    lexer::{Token, TokenType},
    typing::{TypeCell, TypeDef},
};
use inkwell::{
    FloatPredicate,
    builder::Builder,
    context::Context,
    module::Module,
    types::{BasicType, BasicTypeEnum},
    values::BasicValueEnum,
};
use std::rc::Rc;

pub struct LLVMGenerator<'a, 'ctx> {
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub module: &'a Module<'ctx>,
}

impl<'a, 'ctx> LLVMGenerator<'a, 'ctx> {
    fn llvm_type(&self, ty: &TypeDef) -> BasicTypeEnum<'ctx> {
        match ty {
            TypeDef::Native(name) => match *name {
                "f64" => self.context.f64_type().into(),
                "bool" => self.context.bool_type().into(),
                _ => todo!(),
            },

            TypeDef::Array(inner) => {
                let inner_ty = self.llvm_type(&inner);
                // TODO: add array size
                inner_ty.array_type(16).into()
            }

            TypeDef::Struct { name, members: _ } => self
                .module
                .get_struct_type(name.get_name())
                .expect("Struct not declared")
                .into(),

            _ => unreachable!(),
        }
    }
}

impl<'a, 'ctx> ExprVisitor<'a> for LLVMGenerator<'a, 'ctx> {
    type Output = BasicValueEnum<'ctx>;

    fn visit_unary(&mut self, operator: &'a Token, right: &'a Rc<Expr>) -> Self::Output {
        let rhs = self.visit_expr(right);

        match operator.get_type() {
            TokenType::Add => rhs,
            TokenType::Sub => {
                let zero = self.context.f64_type().const_float(0.0);
                self.builder
                    .build_float_sub(zero, rhs.into_float_value(), "negtmp")
                    .unwrap()
                    .into()
            }
            TokenType::Not => {
                let bool_val = rhs.into_int_value();
                self.builder.build_not(bool_val, "nottmp").unwrap().into()
            }
            _ => unreachable!(),
        }
    }

    fn visit_binary(
        &mut self,
        left: &'a Rc<Expr>,
        operator: &'a Token,
        right: &'a Rc<Expr>,
    ) -> Self::Output {
        let lhs = self.visit_expr(left);
        let rhs = self.visit_expr(right);
        match operator.get_type() {
            TokenType::Add => BasicValueEnum::FloatValue(
                self.builder
                    .build_float_add(lhs.into_float_value(), rhs.into_float_value(), "tmpadd")
                    .unwrap(),
            ),
            TokenType::Sub => BasicValueEnum::FloatValue(
                self.builder
                    .build_float_sub(lhs.into_float_value(), rhs.into_float_value(), "tmpsub")
                    .unwrap(),
            ),
            TokenType::Mul => BasicValueEnum::FloatValue(
                self.builder
                    .build_float_mul(lhs.into_float_value(), rhs.into_float_value(), "tmpmul")
                    .unwrap(),
            ),
            TokenType::Div => BasicValueEnum::FloatValue(
                self.builder
                    .build_float_div(lhs.into_float_value(), rhs.into_float_value(), "tmpdiv")
                    .unwrap(),
            ),
            TokenType::Low => {
                let cmp = self
                    .builder
                    .build_float_compare(
                        inkwell::FloatPredicate::ULT,
                        lhs.into_float_value(),
                        rhs.into_float_value(),
                        "tmpcmp",
                    )
                    .unwrap();

                BasicValueEnum::FloatValue(
                    self.builder
                        .build_unsigned_int_to_float(cmp, self.context.f64_type(), "tmpbool")
                        .unwrap(),
                )
            }
            TokenType::Gre => {
                let cmp = self
                    .builder
                    .build_float_compare(
                        inkwell::FloatPredicate::UGT,
                        lhs.into_float_value(),
                        rhs.into_float_value(),
                        "tmpcmp",
                    )
                    .unwrap();

                BasicValueEnum::FloatValue(
                    self.builder
                        .build_unsigned_int_to_float(cmp, self.context.f64_type(), "tmpbool")
                        .unwrap(),
                )
            }
            _ => unreachable!(),
        }
    }

    fn visit_get(
        &mut self,
        _left: &'a Rc<Expr>,
        _right: &'a Identifier,
        _lookup: &'a TypeCell,
    ) -> Self::Output {
        todo!()
    }

    fn visit_index(
        &mut self,
        _object: &'a Rc<Expr>,
        _index: &'a Rc<Expr>,
        _lookup: &'a TypeCell,
    ) -> Self::Output {
        todo!()
    }

    fn visit_call(&mut self, _callee: &'a Rc<Expr>, arguments: &'a [Rc<Expr>]) -> Self::Output {
        let function = self
            .module
            .get_function("my_function_name") // TODO: retrieve function name from calle expr
            .expect("Expect function is defined");

        let args: Vec<_> = arguments
            .iter()
            .map(|arg| self.visit_expr(arg).into())
            .collect();

        let call = self.builder.build_call(function, &args, "calltmp").unwrap();

        call.try_as_basic_value()
            .expect_basic("Expected return value")
    }

    fn visit_create_array(
        &mut self,
        _array_type: &'a TypeCell,
        _array_size: &'a Option<Rc<Expr>>,
        _fields: &'a [Rc<Expr>],
    ) -> Self::Output {
        todo!()
    }

    fn visit_create_struct(
        &mut self,
        _struct_name: &'a TypeCell,
        _fields: &'a [(Identifier, Rc<Expr>)],
    ) -> Self::Output {
        todo!()
    }

    fn visit_if(
        &mut self,
        condition: &'a Rc<Expr>,
        if_branch: &'a Rc<Stmt>,
        else_branch: &'a Option<Rc<Stmt>>,
        _expression_result: &'a TypeCell,
    ) -> Self::Output {
        let parent = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();

        let cond = self.visit_expr(condition).into_float_value();

        let zero = self.context.f64_type().const_float(0.0);
        let cond = self
            .builder
            .build_float_compare(FloatPredicate::ONE, cond, zero, "ifcond")
            .unwrap();

        let then_block = self.context.append_basic_block(parent, "then");
        let else_block = self.context.append_basic_block(parent, "else");
        let merge_block = self.context.append_basic_block(parent, "ifcont");

        self.builder
            .build_conditional_branch(cond, then_block, else_block)
            .unwrap();

        // THEN
        self.builder.position_at_end(then_block);
        let then_val = self.visit_stmt(if_branch);
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();
        let then_end = self.builder.get_insert_block().unwrap();

        // ELSE
        self.builder.position_at_end(else_block);
        let else_val = if let Some(stmt) = else_branch {
            self.visit_stmt(stmt)
        } else {
            self.context.f64_type().const_float(0.0).into()
        };
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();
        let else_end = self.builder.get_insert_block().unwrap();

        // MERGE
        self.builder.position_at_end(merge_block);
        let phi = self
            .builder
            .build_phi(self.context.f64_type(), "iftmp")
            .unwrap();

        phi.add_incoming(&[
            (&then_val.into_float_value(), then_end),
            (&else_val.into_float_value(), else_end),
        ]);

        phi.as_basic_value()
    }

    fn visit_literal(&mut self, value: &'a Token) -> Self::Output {
        match value.get_type() {
            TokenType::Number(val) => self.context.f64_type().const_float(*val).into(),
            TokenType::Bool(val) => self
                .context
                .bool_type()
                .const_int(if *val { 1 } else { 0 }, false)
                .into(),
            _ => unreachable!(),
        }
    }

    fn visit_variable(&mut self, _name: &'a Identifier, _var_type: &'a TypeCell) -> Self::Output {
        todo!()
    }
}

impl<'a, 'ctx> StmtVisitor<'a> for LLVMGenerator<'a, 'ctx> {
    type Output = BasicValueEnum<'ctx>;

    fn visit_block(&mut self, _statements: &'a [Rc<Stmt>]) -> Self::Output {
        todo!()
    }

    fn visit_let(
        &mut self,
        _name: &'a Identifier,
        _var_type: &'a TypeCell,
        _initializer: &'a Expr,
    ) -> Self::Output {
        todo!()
    }

    fn visit_return(&mut self, _value: &'a Expr) -> Self::Output {
        todo!()
    }

    fn visit_break(&mut self) -> Self::Output {
        todo!()
    }

    fn visit_while(&mut self, _condition: &'a Expr, _body: &'a Rc<Stmt>) -> Self::Output {
        todo!()
    }

    fn visit_for(
        &mut self,
        _initializer: &'a Rc<Stmt>,
        _condition: &'a Expr,
        _increment: &'a Expr,
        _body: &'a Rc<Stmt>,
    ) -> Self::Output {
        todo!()
    }

    fn visit_expr_stmt(&mut self, _expr: &'a Expr) -> Self::Output {
        todo!()
    }
}
