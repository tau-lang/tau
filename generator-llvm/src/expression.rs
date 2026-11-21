use crate::LlvmCodeGen;
use inkwell::{
    FloatPredicate, IntPredicate,
    values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, PointerValue},
};
use tau_core::{
    ast::{
        Identifier, Unfold,
        expression::{Expr, ExprFolder, ExprKind},
        fold_vec,
        lookup::{CallLookup, Lookup, VarLookup},
        origin::{BinaryOp, Literal, Origin, UnaryOp},
    },
    error::DynResult,
    lexer::Source,
    type_::{Type, Typed, type_var::collapse_links},
};

impl<'a, 'ctx> LlvmCodeGen<'a, 'ctx> {
    /// Obtain the address of an assignable expression.
    fn fold_lvalue(&mut self, expr: Expr<Typed>) -> DynResult<PointerValue<'ctx>> {
        let kind = expr.unfold();
        match kind {
            ExprKind::Variable { lookup, .. } => match lookup.kind() {
                Lookup::Unqualified => self
                    .variables
                    .get(&lookup.name().symbol())
                    .copied()
                    .ok_or_else(|| "Unknown variable".into()),

                Lookup::Qualified(_) => unimplemented!(),
            },

            ExprKind::Unary {
                operator, right, ..
            } if matches!(operator.kind(), UnaryOp::Deref) => {
                let pointer = self.fold_expr(*right)?;

                Ok(pointer.into_pointer_value())
            }

            ExprKind::AccessStruct { left, right, .. } => {
                let left_type = collapse_links(left.type_().clone());
                let struct_name = match left_type.as_ref() {
                    Type::Named { name, .. } => name,
                    _ => return Err("Expected struct expression".into()),
                };

                let index = *self
                    .structs
                    .get(struct_name)
                    .and_then(|fields| fields.get(&right.symbol()))
                    .ok_or("Unknown struct field")?;

                let ptr = self.fold_lvalue(*left)?;
                let struct_type = self
                    .module
                    .get_struct_type(struct_name.as_str())
                    .ok_or("Unknown struct type")?;

                Ok(self
                    .builder
                    .build_struct_gep(struct_type, ptr, index, "structfield")?)
            }

            ExprKind::IndexArray { object, index, .. } => {
                let object_type = collapse_links(object.type_().clone());
                let index = self.fold_expr(*index)?.into_int_value();
                let ptr = self.fold_lvalue(*object)?;

                match object_type.as_ref() {
                    Type::Array { .. } => {
                        let zero = self.context.i32_type().const_zero();

                        Ok(unsafe {
                            self.builder.build_gep(
                                self.fold_type(&object_type),
                                ptr,
                                &[zero, index],
                                "arrayindex",
                            )?
                        })
                    }

                    _ => Err("Cannot index this expression".into()),
                }
            }

            _ => Err("Expression is not assignable".into()),
        }
    }

    /// Create a numeric operation from two basic values.
    fn build_arithmetic(
        &self,
        operator: &BinaryOp,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
        is_float: bool,
    ) -> DynResult<BasicValueEnum<'ctx>> {
        if is_float {
            let left = left.into_float_value();
            let right = right.into_float_value();

            let value = match operator {
                BinaryOp::Add => self.builder.build_float_add(left, right, "add")?,
                BinaryOp::Sub => self.builder.build_float_sub(left, right, "sub")?,
                BinaryOp::Mul => self.builder.build_float_mul(left, right, "mul")?,
                BinaryOp::Div => self.builder.build_float_div(left, right, "div")?,
                BinaryOp::Mod => self.builder.build_float_rem(left, right, "rem")?,
                _ => return Err("Invalid floating-point operation".into()),
            };

            Ok(value.as_basic_value_enum())
        } else {
            let left = left.into_int_value();
            let right = right.into_int_value();

            let value = match operator {
                BinaryOp::Add => self.builder.build_int_add(left, right, "add")?,
                BinaryOp::Sub => self.builder.build_int_sub(left, right, "sub")?,
                BinaryOp::Mul => self.builder.build_int_mul(left, right, "mul")?,
                BinaryOp::Div => self.builder.build_int_signed_div(left, right, "div")?,
                BinaryOp::Mod => self.builder.build_int_signed_rem(left, right, "rem")?,
                _ => return Err("Invalid integer operation".into()),
            };

            Ok(value.as_basic_value_enum())
        }
    }
}

impl<'a, 'ctx> ExprFolder for LlvmCodeGen<'a, 'ctx> {
    type Type = Typed;
    type Output = DynResult<BasicValueEnum<'ctx>>;

    fn fold_unary(
        &mut self,
        operator: Origin<UnaryOp>,
        right: Expr<Self::Type>,
        type_: Self::Type,
    ) -> Self::Output {
        match operator.kind() {
            UnaryOp::Neg => {
                let value = self.fold_expr(right)?;

                if type_.is_float() {
                    Ok(self
                        .builder
                        .build_float_neg(value.into_float_value(), "neg")?
                        .as_basic_value_enum())
                } else {
                    Ok(self
                        .builder
                        .build_int_neg(value.into_int_value(), "neg")?
                        .as_basic_value_enum())
                }
            }

            UnaryOp::Not => {
                let value = self.fold_expr(right)?.into_int_value();

                let result = self.builder.build_int_compare(
                    IntPredicate::EQ,
                    value,
                    value.get_type().const_zero(),
                    "not",
                )?;

                Ok(self.as_bool(result))
            }

            UnaryOp::Ref => {
                let ptr = self.fold_lvalue(right)?;
                Ok(ptr.as_basic_value_enum())
            }

            UnaryOp::Deref => {
                let pointer = self.fold_expr(right)?.into_pointer_value();

                Ok(self
                    .builder
                    .build_load(self.fold_type(&type_), pointer, "deref")?)
            }
        }
    }

    fn fold_binary(
        &mut self,
        left: Expr<Self::Type>,
        operator: Origin<BinaryOp>,
        right: Expr<Self::Type>,
        type_: Self::Type,
    ) -> Self::Output {
        let op = operator.kind();

        // Assignment operators must operate on an address.
        match op {
            BinaryOp::Set
            | BinaryOp::SetAdd
            | BinaryOp::SetSub
            | BinaryOp::SetMul
            | BinaryOp::SetDiv => {
                let ptr = self.fold_lvalue(left)?;

                let value = if matches!(op, BinaryOp::Set) {
                    self.fold_expr(right)?
                } else {
                    let current =
                        self.builder
                            .build_load(self.fold_type(&type_), ptr, "oldvalue")?;

                    let rhs = self.fold_expr(right)?;

                    let arithmetic_op = match op {
                        BinaryOp::SetAdd => BinaryOp::Add,
                        BinaryOp::SetSub => BinaryOp::Sub,
                        BinaryOp::SetMul => BinaryOp::Mul,
                        BinaryOp::SetDiv => BinaryOp::Div,
                        _ => unreachable!(),
                    };

                    self.build_arithmetic(&arithmetic_op, current, rhs, type_.is_float())?
                };

                self.builder.build_store(ptr, value)?;

                return Ok(value);
            }

            _ => {}
        }

        let is_float = left.type_().is_float();
        let left = self.fold_expr(left)?;
        let right = self.fold_expr(right)?;

        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                self.build_arithmetic(op, left, right, type_.is_float())
            }

            BinaryOp::Eq
            | BinaryOp::Neq
            | BinaryOp::Low
            | BinaryOp::Leq
            | BinaryOp::Gre
            | BinaryOp::Geq => {
                let result = if is_float {
                    let predicate = match op {
                        BinaryOp::Eq => FloatPredicate::OEQ,
                        BinaryOp::Neq => FloatPredicate::ONE,
                        BinaryOp::Low => FloatPredicate::OLT,
                        BinaryOp::Leq => FloatPredicate::OLE,
                        BinaryOp::Gre => FloatPredicate::OGT,
                        BinaryOp::Geq => FloatPredicate::OGE,
                        _ => unreachable!(),
                    };
                    self.builder.build_float_compare(
                        predicate,
                        left.into_float_value(),
                        right.into_float_value(),
                        "cmp",
                    )?
                } else {
                    let predicate = match op {
                        BinaryOp::Eq => IntPredicate::EQ,
                        BinaryOp::Neq => IntPredicate::NE,
                        BinaryOp::Low => IntPredicate::SLT,
                        BinaryOp::Leq => IntPredicate::SLE,
                        BinaryOp::Gre => IntPredicate::SGT,
                        BinaryOp::Geq => IntPredicate::SGE,
                        _ => unreachable!(),
                    };
                    self.builder.build_int_compare(
                        predicate,
                        left.into_int_value(),
                        right.into_int_value(),
                        "cmp",
                    )?
                };

                Ok(self.as_bool(result))
            }

            BinaryOp::And | BinaryOp::Or => {
                let left = left.into_int_value();
                let right = right.into_int_value();

                let result = match op {
                    BinaryOp::And => self.builder.build_and(left, right, "and")?,
                    BinaryOp::Or => self.builder.build_or(left, right, "or")?,
                    _ => unreachable!(),
                };

                Ok(result.as_basic_value_enum())
            }

            _ => Err("Unsupported binary operation".into()),
        }
    }

    fn fold_access_struct(
        &mut self,
        left: Expr<Self::Type>,
        right: Identifier,
        type_: Self::Type,
    ) -> Self::Output {
        let ptr = self.fold_lvalue(Expr::new(ExprKind::AccessStruct {
            left: Box::new(left),
            right,
            type_: type_.clone(),
        }))?;

        Ok(self
            .builder
            .build_load(self.fold_type(&type_), ptr, "structload")?)
    }

    fn fold_index_array(
        &mut self,
        object: Expr<Self::Type>,
        index: Expr<Self::Type>,
        _: Source,
        type_: Self::Type,
    ) -> Self::Output {
        let object_type = object.type_().clone();
        let ptr = self.fold_lvalue(object)?;
        let index = self.fold_expr(index)?.into_int_value();

        let zero = self.context.i32_type().const_zero();

        let element_ptr = unsafe {
            self.builder.build_gep(
                self.fold_type(&object_type),
                ptr,
                &[zero, index],
                "arrayelement",
            )?
        };

        Ok(self
            .builder
            .build_load(self.fold_type(&type_), element_ptr, "arrayload")?)
    }

    fn fold_call(
        &mut self,
        lookup: CallLookup<Self::Type>,
        arguments: Vec<Expr<Self::Type>>,
        _: Source,
        _type_: Self::Type,
    ) -> Self::Output {
        match self.get_function(lookup.name().symbol().as_str()) {
            Some(fun) => {
                let arguments: Vec<BasicMetadataValueEnum> =
                    fold_vec(arguments, |argument| self.fold_expr(argument))?
                        .into_iter()
                        .map(|val| val.into())
                        .collect();
                match self
                    .builder
                    .build_call(fun, arguments.as_slice(), "tmp")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                {
                    Some(value) => Ok(value),
                    // Void functions
                    None => Ok(self.context.i8_type().const_zero().as_basic_value_enum()),
                }
            }
            None => Err(format!("Unknown function {}", lookup.name()).into()),
        }
    }

    fn fold_create_array(
        &mut self,
        fields: Vec<Expr<Self::Type>>,
        _: Source,
        type_: Typed,
    ) -> Self::Output {
        let array_type = self.fold_type(&type_).into_array_type();

        let mut aggregate = array_type.const_zero();

        for (index, field) in fields.into_iter().enumerate() {
            let value = self.fold_expr(field)?;

            aggregate = self
                .builder
                .build_insert_value(aggregate, value, index as u32, "index")?
                .into_array_value();
        }

        Ok(aggregate.as_basic_value_enum())
    }

    fn fold_create_struct(
        &mut self,
        fields: Vec<(Identifier, Expr<Self::Type>)>,
        _: Source,
        type_: Typed,
    ) -> Self::Output {
        let struct_name = match type_.as_ref() {
            Type::Named { name, .. } => name,
            _ => return Err("Expected struct type".into()),
        };

        let struct_type = self
            .module
            .get_struct_type(struct_name.as_str())
            .ok_or("Unknown struct type")?;

        let mut aggregate = struct_type.const_zero();

        for (name, expr) in fields {
            let struct_fields = self.structs.get(struct_name).ok_or("Unknown struct")?;
            let index = *struct_fields
                .get(&name.symbol())
                .ok_or("Unknown struct field")?;

            let value = self.fold_expr(expr)?;

            // Insert the init into the struct
            aggregate = self
                .builder
                .build_insert_value(aggregate, value, index, "insert")?
                .into_struct_value();
        }

        Ok(aggregate.as_basic_value_enum())
    }

    fn fold_ternary(
        &mut self,
        condition: Expr<Self::Type>,
        if_branch: Expr<Self::Type>,
        else_branch: Expr<Self::Type>,
        _: Source,
        type_: Self::Type,
    ) -> Self::Output {
        let parent = self.fn_value();

        let condition = self.fold_expr(condition)?;
        let condition = self.as_condition(condition);

        let if_block = self.context.append_basic_block(parent, "ternary.if");
        let else_block = self.context.append_basic_block(parent, "ternary.else");
        let merge_block = self.context.append_basic_block(parent, "ternary.end");

        self.builder
            .build_conditional_branch(condition, if_block, else_block)?;

        // True branch
        self.builder.position_at_end(if_block);

        let if_value = self.fold_expr(if_branch)?;
        let if_end = self.builder.get_insert_block().unwrap();

        self.builder.build_unconditional_branch(merge_block)?;

        // False branch
        self.builder.position_at_end(else_block);

        let else_value = self.fold_expr(else_branch)?;
        let else_end = self.builder.get_insert_block().unwrap();

        self.builder.build_unconditional_branch(merge_block)?;

        // Merge
        self.builder.position_at_end(merge_block);

        let phi = self.builder.build_phi(self.fold_type(&type_), "ternary")?;

        phi.add_incoming(&[(&if_value, if_end), (&else_value, else_end)]);

        Ok(phi.as_basic_value())
    }

    fn fold_literal(&mut self, value: Origin<Literal>, type_: Self::Type) -> Self::Output {
        match value.kind() {
            Literal::Int(value) => Ok(self
                .fold_type(&type_)
                .into_int_type()
                .const_int(*value, true)
                .as_basic_value_enum()),
            Literal::Float(value) => Ok(self
                .fold_type(&type_)
                .into_float_type()
                .const_float(*value)
                .as_basic_value_enum()),
            Literal::String(value) => Ok(self
                .builder
                .build_global_string_ptr(value, "string")?
                .as_pointer_value()
                .as_basic_value_enum()),
            Literal::Bool(value) => Ok(self
                .context
                .i8_type()
                .const_int(*value as u64, false)
                .as_basic_value_enum()),
        }
    }

    fn fold_variable(&mut self, lookup: VarLookup, type_: Self::Type) -> Self::Output {
        let ptr_value = match lookup.kind() {
            Lookup::Unqualified => self.variables.get(&lookup.name().symbol()).unwrap(),
            Lookup::Qualified(_) => todo!(),
        };
        Ok(self
            .builder
            .build_load(self.fold_type(&type_), *ptr_value, lookup.name().name())?)
    }
}
