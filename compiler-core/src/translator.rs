//! Lowers the ast to an immediate representation (ir)

use crate::{
    ast::{
        TypedAst,
        declaration::{DeclFolder, FuncDecl, StructDecl},
        expression::{Expr, ExprFolder},
        identifier::Identifier,
        lookup::{CallLookup, VarLookup},
        origin::{BinaryOp, Literal, Origin, UnaryOp},
        statement::{Stmt, StmtFolder},
    },
    ir::{
        aggregate::Aggregate,
        data::DataItem,
        emitter::Emitter,
        instruction::Cmp,
        linkage::Linkage,
        value::{Value, ValueKind},
    },
    lexer::Source,
    type_::{Type, prelude::*},
};
use std::collections::HashMap;
use symbol_table::{GlobalSymbol, static_symbol};

pub type Scope = HashMap<GlobalSymbol, Value>;

/// Lowers an AST to IR. The translator directly calls the backend code
/// generator over the emitter trait.
pub struct Translator<'a, E: Emitter> {
    emitter: &'a mut E,

    /// Values defined in the current function together with the name of the
    /// block.
    scopes: Vec<Scope>,

    /// End labels for each block. They are the jmp destination for the break
    /// statement.
    loop_labels: Vec<GlobalSymbol>,

    data_counter: usize,
    instr_counter: usize,
    block_counter: usize,
}

impl<'a, E: Emitter> Translator<'a, E> {
    /// Creates a new translator from the given emitter.
    pub fn new(emitter: &'a mut E) -> Self {
        Self {
            emitter,
            scopes: Vec::new(),
            loop_labels: Vec::new(),
            data_counter: 0,
            instr_counter: 0,
            block_counter: 0,
        }
    }

    /// Fold the ast and lower it to ir. This will both consume the ast and the
    /// translator.
    pub fn fold_ast(mut self, ast: TypedAst) {
        for decl in ast {
            self.fold_decl(decl);
        }
    }

    fn substitute(&mut self, name: GlobalSymbol, value: Value) {
        self.scopes
            .last_mut()
            .expect("cannot substitute outside a scope")
            .insert(name, value);
    }

    fn resolve_var(&self, name: &GlobalSymbol, type_: Typed) -> Value {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .cloned()
            .unwrap_or_else(|| Value::new(ValueKind::Global(*name), type_))
    }

    fn begin_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop().expect("unbalanced scopes");
    }

    fn begin_loop(&mut self, exit_label: GlobalSymbol) {
        self.loop_labels.push(exit_label);
    }

    fn end_loop(&mut self) {
        self.loop_labels.pop().expect("unbalanced loops");
    }

    fn loop_exit_label(&self) -> GlobalSymbol {
        *self.loop_labels.last().expect("break outside loop")
    }

    fn begin_block(&mut self, label: GlobalSymbol) {
        self.emitter.begin_block(label);
    }

    fn end_block(&mut self) {
        self.emitter.end_block();
    }

    fn fold_scoped_statements(&mut self, statements: Vec<Stmt<Typed>>) {
        self.begin_scope();

        for stmt in statements {
            self.fold_stmt(stmt);
        }

        self.end_scope();
    }

    fn auto_load(&mut self, value: Value) -> Value {
        if matches!(value.kind(), ValueKind::AutoLoad(_)) {
            let load = self.emitter.emit_instr_load(value.type_(), &value);
            self.save_temp_instr(value.type_().clone(), load)
        } else {
            value
        }
    }

    fn save_var_instr(&mut self, name: GlobalSymbol, type_: Typed, init: Value) -> Value {
        if type_.as_ref().is_aggregate() {
            return init;
        }

        let address = GlobalSymbol::from(format!("{}.addr", name));
        let dest = Value::new(ValueKind::AutoLoad(address), type_.clone());

        let alloc = self.emitter.emit_instr_alloca(type_.clone(), None);

        let pointer_type = Typed::new(Type::Ptr {
            type_: type_.clone(),
            unchecked: true,
        });

        self.emitter.emit_stmt_assign(address, &pointer_type, alloc);

        self.emitter.emit_stmt_store(&dest, &init);

        dest
    }

    fn save_temp_instr(&mut self, type_: Typed, instr: E::Instr) -> Value {
        let name = self.new_instr_name();

        let value = Value::new(ValueKind::Temporary(name), type_.clone());

        self.emitter.emit_stmt_assign(name, &type_, instr);

        value
    }

    fn new_instr_name(&mut self) -> GlobalSymbol {
        Self::next_name("ti", &mut self.instr_counter)
    }

    fn new_data_name(&mut self) -> GlobalSymbol {
        Self::next_name("td", &mut self.data_counter)
    }

    fn new_block_name(&mut self) -> GlobalSymbol {
        Self::next_name("tb", &mut self.block_counter)
    }

    fn next_name(prefix: &str, counter: &mut usize) -> GlobalSymbol {
        let name = format!("{prefix}{}", *counter).into();
        *counter += 1;
        name
    }

    fn reset_counters(&mut self) {
        self.instr_counter = 0;
        self.block_counter = 0;
    }
}

impl<'a, E: Emitter> DeclFolder for Translator<'a, E> {
    type Type = Typed;
    type Output = ();

    fn fold_import(&mut self, _: Vec<Identifier>, _: Source) -> Self::Output {}

    fn fold_struct(&mut self, struct_: StructDecl, _: Source) -> Self::Output {
        let name = struct_.name.symbol();
        let items = struct_.fields.into_iter().map(|(_, v)| v.clone()).collect();
        let aggregate = Aggregate::Struct { name, items };
        self.emitter.emit_type_def(aggregate);
    }

    fn fold_function(&mut self, func: FuncDecl<Self::Type>, _: Source) -> Self::Output {
        if func.modifiers.is_extern {
            return;
        }

        let linkage = Linkage { exported: true };
        let name = func.name.symbol();
        let args = func
            .params
            .iter()
            .map(|(k, v)| (k.symbol(), v.clone()))
            .collect();
        let is_void = func.return_type.is_void();
        let return_type = if is_void {
            None
        } else {
            Some(func.return_type)
        };

        self.emitter.begin_func(linkage, name, args, return_type);
        self.emitter.begin_block(static_symbol!("start"));

        self.begin_scope();
        for (old, type_) in func.params {
            let old = old.symbol();
            let value = Value::new(ValueKind::Temporary(old), type_.clone());
            let new = self.save_var_instr(old, type_, value);
            self.substitute(old, new);
        }
        for stmt in func.body {
            self.fold_stmt(stmt);
        }

        // Implicit return for void functions
        if is_void {
            self.emitter.emit_stmt_ret(None);
        }
        self.end_scope();

        self.emitter.end_block();
        self.emitter.end_func();
        self.reset_counters();
    }

    fn fold_const(
        &mut self,
        _: Identifier,
        _: Typed,
        _: Expr<Self::Type>,
        _: Source,
    ) -> Self::Output {
    }
}

impl<'a, E: Emitter> StmtFolder for Translator<'a, E> {
    type Type = Typed;
    type Output = ();

    fn fold_block(&mut self, statements: Vec<Stmt<Self::Type>>, _: Source) -> Self::Output {
        // This is the name of the next block. We use this as a jmp destination
        // if we need to leave the block earlier.
        let end_label = self.new_block_name();

        self.fold_scoped_statements(statements);

        self.emitter.end_block();
        self.emitter.begin_block(end_label);
    }

    fn fold_let(
        &mut self,
        name: Identifier,
        declared: Option<Typed>,
        initializer: Expr<Self::Type>,
        _: Source,
    ) -> Self::Output {
        let type_ = declared.expect("type infered");
        let name = name.symbol();
        let init = self.fold_expr(initializer);
        let alloc = self.save_var_instr(name, type_, init);
        self.substitute(name, alloc)
    }

    fn fold_return(&mut self, value: Option<Expr<Self::Type>>, _: Source) -> Self::Output {
        let value = value.map(|v| self.fold_expr(v));
        let ret = self.emitter.emit_instr_ret(value);
        self.emitter.emit_stmt_volatile(ret);
    }

    fn fold_break(&mut self, _: Source) -> Self::Output {
        let label = self.loop_exit_label();
        let jmp_end = self.emitter.emit_instr_jmp(label);
        self.emitter.emit_stmt_volatile(jmp_end);
    }

    fn fold_if(
        &mut self,
        condition: Expr<Self::Type>,
        if_branch: Vec<Stmt<Self::Type>>,
        else_branch: Option<Vec<Stmt<Self::Type>>>,
        _: Source,
    ) -> Self::Output {
        let then_label = self.new_block_name();
        let else_label = self.new_block_name();
        let end_label = if else_branch.is_some() {
            self.new_block_name()
        } else {
            else_label
        };
        let false_label = if else_branch.is_some() {
            else_label
        } else {
            end_label
        };

        // Jump if not zero
        let condition = self.fold_expr(condition);
        self.emitter
            .emit_stmt_jnz(condition, then_label, false_label);
        self.end_block();

        // Then branch
        self.begin_block(then_label);
        self.fold_scoped_statements(if_branch);
        self.emitter.emit_stmt_jmp(end_label);

        // Else branch
        if let Some(statements) = else_branch {
            self.end_block();
            self.begin_block(else_label);

            self.fold_scoped_statements(statements);
            self.emitter.emit_stmt_jmp(end_label);
        }

        self.end_block();
        self.begin_block(end_label);
    }

    fn fold_while(
        &mut self,
        condition: Expr<Self::Type>,
        body: Vec<Stmt<Self::Type>>,
        _: Source,
    ) -> Self::Output {
        // Define jump labels
        let check_block = self.new_block_name();
        let while_block = self.new_block_name();
        let end_block = self.new_block_name();

        // Jump if condition != 0
        self.emitter.end_block();
        self.emitter.begin_block(check_block);
        let condition = self.fold_expr(condition);
        self.emitter
            .emit_stmt_jnz(condition, while_block, end_block);

        // Save while block
        self.emitter.begin_block(while_block);
        self.begin_loop(end_block);
        self.fold_scoped_statements(body);

        // Go back to the condition
        self.emitter.emit_stmt_jmp(check_block);
        self.end_loop();
        self.emitter.end_block();

        // Save end block
        self.emitter.begin_block(end_block);
    }

    fn fold_expr_stmt(&mut self, expr: Expr<Self::Type>, _: Source) -> Self::Output {
        // Drop return value is fine.
        self.fold_expr(expr);
    }
}

impl<'a, E: Emitter> ExprFolder for Translator<'a, E> {
    type Type = Typed;
    type Output = Value;

    fn fold_unary(
        &mut self,
        operator: Origin<UnaryOp>,
        right: Expr<Self::Type>,
        type_: Self::Type,
    ) -> Self::Output {
        let right = self.fold_expr(right);
        let right = self.auto_load(right);
        let instr = match operator.kind() {
            UnaryOp::Neg => self.emitter.emit_instr_neg(right),
            UnaryOp::Not => self.emitter.emit_instr_not(right),
            UnaryOp::Ref => {
                todo!()
            }
            UnaryOp::Deref => {
                todo!()
            }
        };
        self.save_temp_instr(type_, instr)
    }

    fn fold_binary(
        &mut self,
        left: Expr<Self::Type>,
        operator: Origin<BinaryOp>,
        right: Expr<Self::Type>,
        type_: Self::Type,
    ) -> Self::Output {
        let load_type = left.type_().clone();
        let left = self.fold_expr(left);
        let right = self.fold_expr(right);
        let right = self.auto_load(right);

        match operator.kind() {
            op @ (BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::And
            | BinaryOp::Or) => {
                let left = self.auto_load(left);
                let comp = match op {
                    BinaryOp::Add => self.emitter.emit_instr_add(left, right),
                    BinaryOp::Sub => self.emitter.emit_instr_sub(left, right),
                    BinaryOp::Mul => self.emitter.emit_instr_mul(left, right),
                    BinaryOp::Div => self.emitter.emit_instr_div(left, right),
                    BinaryOp::Mod => self.emitter.emit_instr_mod(left, right),
                    BinaryOp::And => self.emitter.emit_instr_and(left, right),
                    BinaryOp::Or => self.emitter.emit_instr_or(left, right),
                    _ => unreachable!(),
                };
                self.save_temp_instr(type_, comp)
            }

            BinaryOp::Set => {
                let store = self.emitter.emit_instr_store(&left, &right);
                self.emitter.emit_stmt_volatile(store);
                right
            }
            op @ (BinaryOp::SetAdd | BinaryOp::SetSub | BinaryOp::SetMul | BinaryOp::SetDiv) => {
                let load = self.emitter.emit_instr_load(&load_type, &left);
                let old_value = self.save_temp_instr(type_.clone(), load);
                let comp = match op {
                    BinaryOp::SetAdd => self.emitter.emit_instr_add(old_value, right),
                    BinaryOp::SetSub => self.emitter.emit_instr_sub(old_value, right),
                    BinaryOp::SetMul => self.emitter.emit_instr_mul(old_value, right),
                    BinaryOp::SetDiv => self.emitter.emit_instr_div(old_value, right),
                    _ => unreachable!(),
                };
                let new_value = self.save_temp_instr(type_.clone(), comp);
                let store = self.emitter.emit_instr_store(&left, &new_value);
                self.emitter.emit_stmt_volatile(store);
                new_value
            }
            op @ (BinaryOp::Eq
            | BinaryOp::Neq
            | BinaryOp::Low
            | BinaryOp::Leq
            | BinaryOp::Gre
            | BinaryOp::Geq) => {
                let left = self.auto_load(left);
                let cmp = match op {
                    BinaryOp::Eq => Cmp::Eq,
                    BinaryOp::Neq => Cmp::Neq,
                    BinaryOp::Low => Cmp::Lt,
                    BinaryOp::Leq => Cmp::Leq,
                    BinaryOp::Gre => Cmp::Gt,
                    BinaryOp::Geq => Cmp::Geq,
                    _ => unreachable!(),
                };
                let cmp = self.emitter.emit_instr_cmp(cmp, left, right);
                self.save_temp_instr(type_, cmp)
            }
        }
    }

    fn fold_access_struct(
        &mut self,
        left: Expr<Self::Type>,
        right: Identifier,
        type_: Self::Type,
    ) -> Self::Output {
        // TODO: find offset
        let address_type = left.type_().clone();
        let left = self.fold_expr(left);
        let add = self
            .emitter
            .emit_instr_add(left, Value::new(ValueKind::Const(0), address_type));

        let name = GlobalSymbol::from(format!("{}.addr", right));
        self.emitter.emit_stmt_assign(
            name,
            &Typed::new(Type::Ptr {
                type_: type_.clone(),
                unchecked: true,
            }),
            add,
        );

        Value::new(ValueKind::AutoLoad(name), type_.clone())
    }

    fn fold_index_array(
        &mut self,
        object: Expr<Self::Type>,
        index: Expr<Self::Type>,
        _: Source,
        type_: Self::Type,
    ) -> Self::Output {
        let address_type = object.type_().clone();
        let (_, size) = self.emitter.layout_hint(&type_);
        let object = self.fold_expr(object);
        let index_type = index.type_().clone();
        let index = self.fold_expr(index);
        let offset = self
            .emitter
            .emit_instr_mul(index, Value::new(ValueKind::Const(size), index_type));
        let offset = self.save_temp_instr(address_type.clone(), offset);
        let add = self.emitter.emit_instr_add(object, offset);
        self.save_temp_instr(address_type, add)
    }

    fn fold_call(
        &mut self,
        lookup: CallLookup<Self::Type>,
        arguments: Vec<Expr<Self::Type>>,
        _: Source,
        type_: Self::Type,
    ) -> Self::Output {
        let arguments = arguments
            .into_iter()
            .map(|arg| {
                let adr = self.fold_expr(arg);
                self.auto_load(adr)
            })
            .collect();
        let return_ = self
            .emitter
            .emit_instr_call(lookup.name().symbol(), arguments);
        self.save_temp_instr(type_, return_)
    }

    fn fold_create_array(
        &mut self,
        fields: Vec<Expr<Self::Type>>,
        _: Source,
        type_: Typed,
    ) -> Self::Output {
        let alloc = self.emitter.emit_instr_alloca(type_.clone(), None);
        let alloc = self.save_temp_instr(type_.clone(), alloc);
        let mut offset = 0;
        for init in fields {
            let (align, size) = self.emitter.layout_hint(init.type_());
            let add = self.emitter.emit_instr_add(
                alloc.clone(),
                Value::new(ValueKind::Const(offset), alloc.type_().clone()),
            );
            let location = self.save_temp_instr(alloc.type_().clone(), add);
            let init = self.fold_expr(init);
            let store = self.emitter.emit_instr_store(&location, &init);
            self.emitter.emit_stmt_volatile(store);
            offset += align.max(size);
        }
        alloc
    }

    fn fold_create_struct(
        &mut self,
        fields: Vec<(Identifier, Expr<Self::Type>)>,
        _: Source,
        type_: Typed,
    ) -> Self::Output {
        let ptr_type = Typed::new(Type::Ptr {
            type_: type_.clone(),
            unchecked: false,
        });
        let alloc = self.emitter.emit_instr_alloca(type_.clone(), None);
        let alloc = self.save_temp_instr(ptr_type, alloc);
        let mut offset = 0;
        for (_, init) in fields {
            let (align, size) = self.emitter.layout_hint(init.type_());
            let add = self.emitter.emit_instr_add(
                alloc.clone(),
                Value::new(ValueKind::Const(offset), alloc.type_().clone()),
            );
            let location = self.save_temp_instr(alloc.type_().clone(), add);
            let init = self.fold_expr(init);
            let init = self.auto_load(init);
            let store = self.emitter.emit_instr_store(&location, &init);
            self.emitter.emit_stmt_volatile(store);
            offset += align.max(size);
        }
        alloc
    }

    fn fold_ternary(
        &mut self,
        condition: Expr<Self::Type>,
        if_branch: Expr<Self::Type>,
        else_branch: Expr<Self::Type>,
        _: Source,
        type_: Self::Type,
    ) -> Self::Output {
        let if_block = self.new_block_name();
        let else_block = self.new_block_name();
        let end_block = self.new_block_name();

        let condition = self.fold_expr(condition);
        let con_jmp = self.emitter.emit_instr_jnz(condition, if_block, else_block);
        self.emitter.emit_stmt_volatile(con_jmp);
        self.emitter.end_block();

        // Save if block
        self.emitter.begin_block(if_block);
        self.begin_scope();
        let if_branch = self.fold_expr(if_branch);
        self.emitter.emit_stmt_jmp(end_block);
        self.end_scope();
        self.emitter.end_block();

        // Save else block
        self.emitter.begin_block(else_block);
        self.begin_scope();
        let else_branch = self.fold_expr(else_branch);
        let else_jmp = self.emitter.emit_instr_jmp(end_block);
        self.emitter.emit_stmt_volatile(else_jmp);
        self.end_scope();
        self.emitter.end_block();

        // Phi instruction
        self.emitter.begin_block(end_block);
        let phi = self
            .emitter
            .emit_instr_phi(vec![(if_block, if_branch), (else_block, else_branch)]);
        self.save_temp_instr(type_, phi)
    }

    fn fold_literal(&mut self, value: Origin<Literal>, type_: Self::Type) -> Self::Output {
        let kind = match value.kind() {
            Literal::Int(value) => ValueKind::Const(*value),
            Literal::Float(value) => ValueKind::Const((*value).to_bits()),
            Literal::Bool(value) => ValueKind::Const(*value as u64),
            Literal::String(value) => {
                let linkage = Linkage { exported: false };
                let name = self.new_data_name();
                let items = vec![
                    (i8(), DataItem::Str(value.into())),
                    // C ABI strings are null terminated.
                    (i8(), DataItem::Const(0)),
                ];

                self.emitter.emit_data_def(linkage, name, items);

                ValueKind::Global(name)
            }
        };
        Value::new(kind, type_)
    }

    fn fold_variable(&mut self, lookup: VarLookup, type_: Self::Type) -> Self::Output {
        // TODO: resolve qualified lookup
        self.resolve_var(&lookup.name().symbol(), type_)
    }
}
