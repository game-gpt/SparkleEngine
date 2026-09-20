//! Oaks `oak-ruby` AST → `spark-vm` 字节码。
//!
//! 解析在 `RubyBuilder`。本文件下沉 RGSS 常用子集：
//! 方法 / 类方法（`Class_method` + `self`）、全局 / 实例变量、
//! `Foo.new`、接收者调用（`Send`）、数组、分支与循环。

use std::collections::{HashMap, HashSet};

use oak_ruby::ast::{ExpressionNode, LiteralNode, RubyRoot, StatementNode};
use spark_vm::{FuncProto, Module, Op};

pub(crate) fn compile_root(root: &RubyRoot, native_names: &[&str]) -> Result<Module, String> {
    let native_set: HashSet<&str> = native_names.iter().copied().collect();
    let mut methods = Vec::new();
    collect_methods(&root.statements, &mut methods, None);

    let mut functions: Vec<FuncProto> = Vec::new();
    let mut fn_index: HashMap<String, usize> = HashMap::new();
    for method in &methods {
        let idx = functions.len();
        fn_index.insert(method.name.clone(), idx);
        functions.push(FuncProto::new(method.name.clone(), method.params.len() as u8));
    }
    let main_idx = functions.len();
    functions.push(FuncProto::new("__main", 0));

    let mut module = Module {
        functions,
        entry: main_idx,
        native_names: Vec::new(),
    };
    for name in native_names {
        module.intern_native(*name);
    }

    for method in &methods {
        let idx = *fn_index.get(&method.name).unwrap();
        compile_method(&mut module, idx, method, &fn_index, &native_set)?;
    }

    {
        let mut ctx = Ctx {
            f: &mut module.functions[main_idx],
            locals: HashMap::new(),
            fn_index: &fn_index,
            native_set: &native_set,
            module_natives: &mut module.native_names,
            loop_breaks: Vec::new(),
        };
        let mut saw_return = false;
        for stmt in &root.statements {
            if matches!(stmt, StatementNode::MethodDef { .. }) {
                continue;
            }
            if matches!(stmt, StatementNode::Return { .. }) {
                saw_return = true;
            }
            compile_stmt(&mut ctx, stmt)?;
        }
        if !saw_return {
            ctx.f.emit(Op::LoadNull);
            ctx.f.emit(Op::Return);
        }
    }

    Ok(module)
}

struct MethodRef {
    name: String,
    params: Vec<String>,
    body: Vec<StatementNode>,
}

fn collect_methods(stmts: &[StatementNode], out: &mut Vec<MethodRef>, class_prefix: Option<&str>) {
    for stmt in stmts {
        match stmt {
            StatementNode::MethodDef {
                name, params, body, ..
            } => {
                let (fname, fparams) = if let Some(prefix) = class_prefix {
                    let mut p = Vec::with_capacity(params.len() + 1);
                    p.push("self".into());
                    p.extend(params.iter().cloned());
                    (format!("{prefix}_{name}"), p)
                } else {
                    (name.clone(), params.clone())
                };
                out.push(MethodRef {
                    name: fname,
                    params: fparams,
                    body: body.clone(),
                });
                collect_methods(body, out, class_prefix);
            }
            StatementNode::ClassDef { name, body, .. } => {
                collect_methods(body, out, Some(name.as_str()));
            }
            StatementNode::If {
                then_body,
                else_body,
                ..
            } => {
                collect_methods(then_body, out, class_prefix);
                if let Some(else_body) = else_body {
                    collect_methods(else_body, out, class_prefix);
                }
            }
            StatementNode::While { body, .. } | StatementNode::Until { body, .. } => {
                collect_methods(body, out, class_prefix);
            }
            StatementNode::For { body, .. } => {
                collect_methods(body, out, class_prefix);
            }
            StatementNode::Expression(ExpressionNode::MethodCall {
                block_body: Some(body),
                ..
            }) => {
                collect_methods(body, out, class_prefix);
            }
            _ => {}
        }
    }
}

fn compile_method(
    module: &mut Module,
    idx: usize,
    method: &MethodRef,
    fn_index: &HashMap<String, usize>,
    native_set: &HashSet<&str>,
) -> Result<(), String> {
    module.functions[idx].locals = method.params.len() as u16;
    let mut locals = HashMap::new();
    for (i, param) in method.params.iter().enumerate() {
        locals.insert(param.clone(), i as u16);
    }
    let mut ctx = Ctx {
        f: &mut module.functions[idx],
        locals,
        fn_index,
        native_set,
        module_natives: &mut module.native_names,
        loop_breaks: Vec::new(),
    };
    compile_block_as_body(&mut ctx, &method.body)?;
    Ok(())
}

struct Ctx<'a> {
    f: &'a mut FuncProto,
    locals: HashMap<String, u16>,
    fn_index: &'a HashMap<String, usize>,
    native_set: &'a HashSet<&'a str>,
    module_natives: &'a mut Vec<String>,
    /// 循环 break 补丁点（相对跳到循环后）。
    loop_breaks: Vec<Vec<usize>>,
}

impl Ctx<'_> {
    fn alloc_local(&mut self, name: &str) -> u16 {
        if let Some(&slot) = self.locals.get(name) {
            return slot;
        }
        let slot = self.locals.len() as u16;
        self.locals.insert(name.to_string(), slot);
        self.f.locals = self.f.locals.max(slot + 1);
        slot
    }

    fn intern_native(&mut self, name: &str) -> u16 {
        if !self.module_natives.iter().any(|n| n == name) {
            self.module_natives.push(name.to_string());
        }
        // CallNative 操作数走本函数字符串池，链接后无需重映射下标。
        self.f.add_string(name.to_string())
    }

    fn try_load_self(&mut self) -> bool {
        if let Some(&slot) = self.locals.get("self") {
            self.f.emit(Op::LoadLocal);
            self.f.emit_u16(slot);
            true
        } else {
            false
        }
    }
}

fn is_constant_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_uppercase())
}

fn is_global_name(name: &str) -> bool {
    name.starts_with('$') || name.starts_with("@@") || is_constant_name(name)
}

fn is_ivar_name(name: &str) -> bool {
    name.starts_with('@') && !name.starts_with("@@")
}

fn setter_flat_name(class_or_recv: &str, method: &str) -> String {
    if let Some(base) = method.strip_suffix('=') {
        format!("{class_or_recv}_{base}_set")
    } else {
        format!("{class_or_recv}_{method}")
    }
}

fn compile_block_as_body(ctx: &mut Ctx<'_>, body: &[StatementNode]) -> Result<(), String> {
    if body.is_empty() {
        ctx.f.emit(Op::LoadNull);
        ctx.f.emit(Op::Return);
        return Ok(());
    }
    let mut saw_return = false;
    for stmt in body {
        if matches!(stmt, StatementNode::MethodDef { .. }) {
            continue;
        }
        if matches!(stmt, StatementNode::Return { .. }) {
            saw_return = true;
        }
        compile_stmt(ctx, stmt)?;
    }
    if !saw_return {
        ctx.f.emit(Op::LoadNull);
        ctx.f.emit(Op::Return);
    }
    Ok(())
}

fn compile_stmt(ctx: &mut Ctx<'_>, stmt: &StatementNode) -> Result<(), String> {
    match stmt {
        StatementNode::MethodDef { .. } => Ok(()),
        StatementNode::ClassDef { body, .. } => {
            for child in body {
                if matches!(child, StatementNode::MethodDef { .. }) {
                    continue;
                }
                compile_stmt(ctx, child)?;
            }
            Ok(())
        }
        StatementNode::Assignment { target, value, .. } => compile_assignment(ctx, target, value),
        StatementNode::Expression(expr) => {
            compile_expr(ctx, expr)?;
            ctx.f.emit(Op::Pop);
            ctx.f.emit_u8(1);
            Ok(())
        }
        StatementNode::Return { value, .. } => {
            match value {
                Some(expr) => compile_expr(ctx, expr)?,
                None => ctx.f.emit(Op::LoadNull),
            }
            ctx.f.emit(Op::Return);
            Ok(())
        }
        StatementNode::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            compile_expr(ctx, condition)?;
            ctx.f.emit(Op::JumpIfFalse);
            let jump_else = ctx.f.len();
            ctx.f.emit_i16(0);
            for stmt in then_body {
                compile_stmt(ctx, stmt)?;
            }
            ctx.f.emit(Op::Jump);
            let jump_end = ctx.f.len();
            ctx.f.emit_i16(0);
            let else_at = ctx.f.len();
            ctx.f
                .patch_i16(jump_else, ((else_at as isize) - ((jump_else + 2) as isize)) as i16);
            if let Some(else_body) = else_body {
                for stmt in else_body {
                    compile_stmt(ctx, stmt)?;
                }
            }
            let end = ctx.f.len();
            ctx.f
                .patch_i16(jump_end, ((end as isize) - ((jump_end + 2) as isize)) as i16);
            Ok(())
        }
        StatementNode::While { condition, body, .. }
        | StatementNode::Until { condition, body, .. } => {
            let invert = matches!(stmt, StatementNode::Until { .. });
            ctx.loop_breaks.push(Vec::new());
            let loop_start = ctx.f.len();
            compile_expr(ctx, condition)?;
            if invert {
                ctx.f.emit(Op::Not);
            }
            ctx.f.emit(Op::JumpIfFalse);
            let jump_end = ctx.f.len();
            ctx.f.emit_i16(0);
            for child in body {
                compile_stmt(ctx, child)?;
            }
            ctx.f.emit(Op::Jump);
            let jump_back = ctx.f.len();
            ctx.f.emit_i16(0);
            let end = ctx.f.len();
            ctx.f
                .patch_i16(jump_end, ((end as isize) - ((jump_end + 2) as isize)) as i16);
            ctx.f.patch_i16(
                jump_back,
                ((loop_start as isize) - ((jump_back + 2) as isize)) as i16,
            );
            patch_loop_breaks(ctx, end);
            Ok(())
        }
        StatementNode::For {
            var,
            iterable,
            body,
            ..
        } => compile_for(ctx, var, iterable, body),
        StatementNode::Break { .. } => {
            if ctx.loop_breaks.is_empty() {
                // 块/`case` 外 break：RGSS 过渡期当空语句，避免整脚本编译失败。
                ctx.f.emit(Op::LoadNull);
                return Ok(());
            }
            ctx.f.emit(Op::Jump);
            let at = ctx.f.len();
            ctx.f.emit_i16(0);
            ctx.loop_breaks.last_mut().unwrap().push(at);
            Ok(())
        }
        StatementNode::Next { .. } | StatementNode::Redo { .. } | StatementNode::Case { .. } => {
            Err("unsupported_statement".into())
        }
    }
}

fn patch_loop_breaks(ctx: &mut Ctx<'_>, end: usize) {
    if let Some(breaks) = ctx.loop_breaks.pop() {
        for at in breaks {
            ctx.f
                .patch_i16(at, ((end as isize) - ((at + 2) as isize)) as i16);
        }
    }
}

fn compile_assignment(ctx: &mut Ctx<'_>, target: &str, value: &ExpressionNode) -> Result<(), String> {
    if is_ivar_name(target) {
        if ctx.try_load_self() {
            compile_expr(ctx, value)?;
            let slot = ctx.f.add_string(target.to_string());
            ctx.f.emit(Op::SetField);
            ctx.f.emit_u16(slot);
            ctx.f.emit(Op::Pop);
            ctx.f.emit_u8(1);
            return Ok(());
        }
        // 类体顶层 `@x`：暂作全局（RGSS 启动期常见）。
        compile_expr(ctx, value)?;
        let slot = ctx.f.add_const_name(target.to_string());
        ctx.f.emit(Op::StoreGlobal);
        ctx.f.emit_u16(slot);
        return Ok(());
    }
    compile_expr(ctx, value)?;
    if is_global_name(target) {
        let slot = ctx.f.add_const_name(target.to_string());
        ctx.f.emit(Op::StoreGlobal);
        ctx.f.emit_u16(slot);
    } else {
        let slot = ctx.alloc_local(target);
        ctx.f.emit(Op::StoreLocal);
        ctx.f.emit_u16(slot);
    }
    Ok(())
}

fn compile_expr(ctx: &mut Ctx<'_>, expr: &ExpressionNode) -> Result<(), String> {
    match expr {
        ExpressionNode::Identifier { name, .. } => compile_name_load(ctx, name)?,
        ExpressionNode::Literal(lit) => compile_literal(ctx, lit)?,
        ExpressionNode::UnaryOp {
            operator, operand, ..
        } => {
            compile_expr(ctx, operand)?;
            match operator.as_str() {
                "-" => ctx.f.emit(Op::Neg),
                "!" | "not" | "~" => ctx.f.emit(Op::Not),
                _ => {}
            }
        }
        ExpressionNode::MethodCall {
            receiver,
            method,
            args,
            block_params,
            block_body,
            ..
        } => compile_method_call(
            ctx,
            receiver.as_deref(),
            method,
            args,
            block_params,
            block_body.as_deref(),
        )?,
        ExpressionNode::BinaryOp {
            left,
            operator,
            right,
            ..
        } => {
            if operator == "&&" {
                return compile_and(ctx, left, right);
            }
            if operator == "||" {
                return compile_or(ctx, left, right);
            }
            if operator == ".." || operator == "..." {
                // 范围：压成二元组数组 [start, end]（含端点约定由 for 解释）。
                compile_expr(ctx, left)?;
                compile_expr(ctx, right)?;
                ctx.f.emit(Op::NewArray);
                ctx.f.emit_u8(2);
                return Ok(());
            }
            compile_expr(ctx, left)?;
            compile_expr(ctx, right)?;
            let op = match operator.as_str() {
                "+" => Op::Add,
                "-" => Op::Sub,
                "*" => Op::Mul,
                "/" => Op::Div,
                "**" => {
                    let slot = ctx.intern_native("pow");
                    ctx.f.emit(Op::CallNative);
                    ctx.f.emit_u16(slot);
                    ctx.f.emit_u8(2);
                    return Ok(());
                }
                "%" => Op::Mod,
                "==" => Op::Eq,
                "!=" => Op::Ne,
                "<" => Op::Lt,
                "<=" => Op::Le,
                ">" => Op::Gt,
                ">=" => Op::Ge,
                _ => return Err(format!("unsupported_binop:{operator}")),
            };
            ctx.f.emit(op);
        }
        ExpressionNode::Array { elements, .. } => {
            for el in elements {
                compile_expr(ctx, el)?;
            }
            ctx.f.emit(Op::NewArray);
            ctx.f.emit_u8(elements.len() as u8);
        }
        ExpressionNode::Hash { pairs, .. } => {
            ctx.f.emit(Op::NewTable);
            for (k, v) in pairs {
                ctx.f.emit(Op::Dup);
                match k {
                    ExpressionNode::Literal(LiteralNode::Symbol { value, .. })
                    | ExpressionNode::Literal(LiteralNode::String { value, .. })
                    | ExpressionNode::Identifier { name: value, .. } => {
                        compile_expr(ctx, v)?;
                        let slot = ctx.f.add_string(value.clone());
                        ctx.f.emit(Op::SetField);
                        ctx.f.emit_u16(slot);
                        ctx.f.emit(Op::Pop);
                        ctx.f.emit_u8(1);
                    }
                    _ => return Err("unsupported_hash_key".into()),
                }
            }
        }
    }
    Ok(())
}

fn compile_name_load(ctx: &mut Ctx<'_>, name: &str) -> Result<(), String> {
    if name == "self" {
        if ctx.try_load_self() {
            return Ok(());
        }
        ctx.f.emit(Op::LoadNull);
        return Ok(());
    }
    if is_ivar_name(name) {
        if ctx.try_load_self() {
            let slot = ctx.f.add_string(name.to_string());
            ctx.f.emit(Op::GetField);
            ctx.f.emit_u16(slot);
            return Ok(());
        }
        let slot = ctx.f.add_const_name(name.to_string());
        ctx.f.emit(Op::LoadGlobal);
        ctx.f.emit_u16(slot);
        return Ok(());
    }
    if let Some(&slot) = ctx.locals.get(name) {
        ctx.f.emit(Op::LoadLocal);
        ctx.f.emit_u16(slot);
        return Ok(());
    }
    if is_global_name(name) {
        let slot = ctx.f.add_const_name(name.to_string());
        ctx.f.emit(Op::LoadGlobal);
        ctx.f.emit_u16(slot);
        return Ok(());
    }
    if let Some(&index) = ctx.fn_index.get(name) {
        let slot = ctx.f.add_const_func(index as u32);
        ctx.f.emit(Op::LoadConst);
        ctx.f.emit_u16(slot);
        return Ok(());
    }
    let slot = ctx.f.add_const_name(name.to_string());
    ctx.f.emit(Op::LoadGlobal);
    ctx.f.emit_u16(slot);
    Ok(())
}

fn compile_method_call(
    ctx: &mut Ctx<'_>,
    receiver: Option<&ExpressionNode>,
    method: &str,
    args: &[ExpressionNode],
    block_params: &[String],
    block_body: Option<&[StatementNode]>,
) -> Result<(), String> {
    // `loop do ... end`
    if method == "loop" && receiver.is_none() {
        if let Some(body) = block_body {
            return compile_infinite_loop(ctx, body);
        }
    }
    // `arr.each {|x| ...}`
    if method == "each" {
        if let Some(body) = block_body {
            let param = block_params.first().map(|s| s.as_str()).unwrap_or("_");
            if let Some(recv) = receiver {
                return compile_each(ctx, recv, param, body);
            }
        }
    }

    // `Foo.new(...)` → 分配带 `__class` 的表，再 `Send initialize`。
    if method == "new" {
        if let Some(ExpressionNode::Identifier { name, .. }) = receiver {
            if is_constant_name(name) || name.contains("::") {
                return compile_class_new(ctx, name, args);
            }
        }
    }

    // 常量 / 模块函数：`Graphics.freeze` / `Font.default_name=` / `RPG::Cache.title`
    if let Some(recv) = receiver {
        if let Some(path) = constant_recv_path(recv) {
            let flat = setter_flat_name(&path.replace("::", "_"), method);
            return compile_call(ctx, &flat, args);
        }
    }

    // 实例调用：压入接收者后 `Send`。
    if let Some(recv) = receiver {
        compile_expr(ctx, recv)?;
        for arg in args {
            compile_expr(ctx, arg)?;
        }
        let method_name = method.to_string();
        let slot = ctx.f.add_string(method_name);
        ctx.f.emit(Op::Send);
        ctx.f.emit_u16(slot);
        ctx.f.emit_u8(args.len() as u8);
        return Ok(());
    }

    compile_call(ctx, method, args)
}

/// `Graphics` / `RPG::Cache` / 嵌套 `RPG.Cache` 常量路径。
fn constant_recv_path(expr: &ExpressionNode) -> Option<String> {
    match expr {
        ExpressionNode::Identifier { name, .. } => {
            if is_constant_name(name) || name.contains("::") {
                Some(name.clone())
            } else {
                None
            }
        }
        ExpressionNode::MethodCall {
            receiver: Some(recv),
            method,
            args,
            block_body: None,
            ..
        } if args.is_empty() && is_constant_name(method) => {
            let base = constant_recv_path(recv)?;
            Some(format!("{base}::{method}"))
        }
        _ => None,
    }
}

fn compile_infinite_loop(ctx: &mut Ctx<'_>, body: &[StatementNode]) -> Result<(), String> {
    ctx.loop_breaks.push(Vec::new());
    let loop_start = ctx.f.len();
    for child in body {
        compile_stmt(ctx, child)?;
    }
    ctx.f.emit(Op::Jump);
    let jump_back = ctx.f.len();
    ctx.f.emit_i16(0);
    let end = ctx.f.len();
    ctx.f.patch_i16(
        jump_back,
        ((loop_start as isize) - ((jump_back + 2) as isize)) as i16,
    );
    patch_loop_breaks(ctx, end);
    ctx.f.emit(Op::LoadNull);
    Ok(())
}

fn compile_for(
    ctx: &mut Ctx<'_>,
    var: &str,
    iterable: &ExpressionNode,
    body: &[StatementNode],
) -> Result<(), String> {
    // for i in a..b → 数值范围；其它可迭代暂当数组 each。
    if let ExpressionNode::BinaryOp {
        left,
        operator,
        right,
        ..
    } = iterable
    {
        if operator == ".." || operator == "..." {
            let i_slot = ctx.alloc_local(var);
            let end_tmp = ctx.alloc_local("__for_end");
            compile_expr(ctx, left)?;
            ctx.f.emit(Op::StoreLocal);
            ctx.f.emit_u16(i_slot);
            compile_expr(ctx, right)?;
            ctx.f.emit(Op::StoreLocal);
            ctx.f.emit_u16(end_tmp);
            ctx.loop_breaks.push(Vec::new());
            let loop_start = ctx.f.len();
            ctx.f.emit(Op::LoadLocal);
            ctx.f.emit_u16(i_slot);
            ctx.f.emit(Op::LoadLocal);
            ctx.f.emit_u16(end_tmp);
            ctx.f.emit(Op::Le);
            ctx.f.emit(Op::JumpIfFalse);
            let jump_end = ctx.f.len();
            ctx.f.emit_i16(0);
            for child in body {
                compile_stmt(ctx, child)?;
            }
            ctx.f.emit(Op::LoadLocal);
            ctx.f.emit_u16(i_slot);
            let one = ctx.f.add_const_number(1.0);
            ctx.f.emit(Op::LoadConst);
            ctx.f.emit_u16(one);
            ctx.f.emit(Op::Add);
            ctx.f.emit(Op::StoreLocal);
            ctx.f.emit_u16(i_slot);
            ctx.f.emit(Op::Jump);
            let jump_back = ctx.f.len();
            ctx.f.emit_i16(0);
            let end = ctx.f.len();
            ctx.f
                .patch_i16(jump_end, ((end as isize) - ((jump_end + 2) as isize)) as i16);
            ctx.f.patch_i16(
                jump_back,
                ((loop_start as isize) - ((jump_back + 2) as isize)) as i16,
            );
            patch_loop_breaks(ctx, end);
            return Ok(());
        }
    }
    compile_each(ctx, iterable, var, body)
}

fn compile_each(
    ctx: &mut Ctx<'_>,
    recv: &ExpressionNode,
    param: &str,
    body: &[StatementNode],
) -> Result<(), String> {
    let arr_slot = ctx.alloc_local("__each_arr");
    let i_slot = ctx.alloc_local("__each_i");
    let len_slot = ctx.alloc_local("__each_len");
    let p_slot = ctx.alloc_local(param);
    compile_expr(ctx, recv)?;
    ctx.f.emit(Op::StoreLocal);
    ctx.f.emit_u16(arr_slot);
    let zero = ctx.f.add_const_number(0.0);
    ctx.f.emit(Op::LoadConst);
    ctx.f.emit_u16(zero);
    ctx.f.emit(Op::StoreLocal);
    ctx.f.emit_u16(i_slot);
    // len = arr.size
    ctx.f.emit(Op::LoadLocal);
    ctx.f.emit_u16(arr_slot);
    let size_s = ctx.f.add_string("size");
    ctx.f.emit(Op::Send);
    ctx.f.emit_u16(size_s);
    ctx.f.emit_u8(0);
    ctx.f.emit(Op::StoreLocal);
    ctx.f.emit_u16(len_slot);
    ctx.loop_breaks.push(Vec::new());
    let loop_start = ctx.f.len();
    ctx.f.emit(Op::LoadLocal);
    ctx.f.emit_u16(i_slot);
    ctx.f.emit(Op::LoadLocal);
    ctx.f.emit_u16(len_slot);
    ctx.f.emit(Op::Lt);
    ctx.f.emit(Op::JumpIfFalse);
    let jump_end = ctx.f.len();
    ctx.f.emit_i16(0);
    // param = arr[i]
    ctx.f.emit(Op::LoadLocal);
    ctx.f.emit_u16(arr_slot);
    ctx.f.emit(Op::LoadLocal);
    ctx.f.emit_u16(i_slot);
    let idx_s = ctx.f.add_string("[]");
    ctx.f.emit(Op::Send);
    ctx.f.emit_u16(idx_s);
    ctx.f.emit_u8(1);
    ctx.f.emit(Op::StoreLocal);
    ctx.f.emit_u16(p_slot);
    for child in body {
        compile_stmt(ctx, child)?;
    }
    ctx.f.emit(Op::LoadLocal);
    ctx.f.emit_u16(i_slot);
    let one = ctx.f.add_const_number(1.0);
    ctx.f.emit(Op::LoadConst);
    ctx.f.emit_u16(one);
    ctx.f.emit(Op::Add);
    ctx.f.emit(Op::StoreLocal);
    ctx.f.emit_u16(i_slot);
    ctx.f.emit(Op::Jump);
    let jump_back = ctx.f.len();
    ctx.f.emit_i16(0);
    let end = ctx.f.len();
    ctx.f
        .patch_i16(jump_end, ((end as isize) - ((jump_end + 2) as isize)) as i16);
    ctx.f.patch_i16(
        jump_back,
        ((loop_start as isize) - ((jump_back + 2) as isize)) as i16,
    );
    patch_loop_breaks(ctx, end);
    ctx.f.emit(Op::LoadNull);
    Ok(())
}

fn compile_class_new(ctx: &mut Ctx<'_>, class_name: &str, args: &[ExpressionNode]) -> Result<(), String> {
    ctx.f.emit(Op::NewTable);
    ctx.f.emit(Op::Dup);
    let class_str = ctx.f.add_string(class_name.to_string());
    ctx.f.emit(Op::LoadString);
    ctx.f.emit_u16(class_str);
    let field = ctx.f.add_string("__class");
    ctx.f.emit(Op::SetField);
    ctx.f.emit_u16(field);
    ctx.f.emit(Op::Pop);
    ctx.f.emit_u8(1);
    let init_fn = format!("{class_name}_initialize");
    if ctx.fn_index.contains_key(&init_fn) {
        ctx.f.emit(Op::Dup);
        for arg in args {
            compile_expr(ctx, arg)?;
        }
        let init = ctx.f.add_string("initialize");
        ctx.f.emit(Op::Send);
        ctx.f.emit_u16(init);
        ctx.f.emit_u8(args.len() as u8);
        ctx.f.emit(Op::Pop);
        ctx.f.emit_u8(1);
    }
    Ok(())
}

fn compile_literal(ctx: &mut Ctx<'_>, lit: &LiteralNode) -> Result<(), String> {
    match lit {
        LiteralNode::Integer { value, .. } => {
            let slot = ctx.f.add_const_number(*value as f64);
            ctx.f.emit(Op::LoadConst);
            ctx.f.emit_u16(slot);
        }
        LiteralNode::Float { value, .. } => {
            let slot = ctx.f.add_const_number(*value);
            ctx.f.emit(Op::LoadConst);
            ctx.f.emit_u16(slot);
        }
        LiteralNode::String { value, .. } | LiteralNode::Symbol { value, .. } => {
            let slot = ctx.f.add_string(value.clone());
            ctx.f.emit(Op::LoadString);
            ctx.f.emit_u16(slot);
        }
        LiteralNode::Boolean { value: true, .. } => ctx.f.emit(Op::LoadTrue),
        LiteralNode::Boolean { value: false, .. } => ctx.f.emit(Op::LoadFalse),
        LiteralNode::Nil { .. } => ctx.f.emit(Op::LoadNull),
    }
    Ok(())
}

fn compile_call(ctx: &mut Ctx<'_>, name: &str, args: &[ExpressionNode]) -> Result<(), String> {
    if name == "print" || name == "puts" || name == "p" {
        if args.is_empty() {
            ctx.f.emit(Op::LoadNull);
            ctx.f.emit(Op::Print);
            return Ok(());
        }
        // 多参数：只打印第一个（RGSS 宿主过渡）。
        compile_expr(ctx, &args[0])?;
        ctx.f.emit(Op::Print);
        return Ok(());
    }
    if ctx.native_set.contains(name) {
        for arg in args {
            compile_expr(ctx, arg)?;
        }
        let slot = ctx.intern_native(name);
        ctx.f.emit(Op::CallNative);
        ctx.f.emit_u16(slot);
        ctx.f.emit_u8(args.len() as u8);
        return Ok(());
    }
    if let Some(&index) = ctx.fn_index.get(name) {
        let slot = ctx.f.add_const_func(index as u32);
        ctx.f.emit(Op::LoadConst);
        ctx.f.emit_u16(slot);
        for arg in args {
            compile_expr(ctx, arg)?;
        }
        ctx.f.emit(Op::Call);
        ctx.f.emit_u8(args.len() as u8);
        return Ok(());
    }
    for arg in args {
        compile_expr(ctx, arg)?;
    }
    let slot = ctx.intern_native(name);
    ctx.f.emit(Op::CallNative);
    ctx.f.emit_u16(slot);
    ctx.f.emit_u8(args.len() as u8);
    Ok(())
}

fn compile_and(ctx: &mut Ctx<'_>, lhs: &ExpressionNode, rhs: &ExpressionNode) -> Result<(), String> {
    compile_expr(ctx, lhs)?;
    ctx.f.emit(Op::JumpIfFalse);
    let jump_false = ctx.f.len();
    ctx.f.emit_i16(0);
    compile_expr(ctx, rhs)?;
    ctx.f.emit(Op::Jump);
    let jump_end = ctx.f.len();
    ctx.f.emit_i16(0);
    let false_at = ctx.f.len();
    ctx.f
        .patch_i16(jump_false, ((false_at as isize) - ((jump_false + 2) as isize)) as i16);
    ctx.f.emit(Op::LoadFalse);
    let end = ctx.f.len();
    ctx.f
        .patch_i16(jump_end, ((end as isize) - ((jump_end + 2) as isize)) as i16);
    Ok(())
}

fn compile_or(ctx: &mut Ctx<'_>, lhs: &ExpressionNode, rhs: &ExpressionNode) -> Result<(), String> {
    compile_expr(ctx, lhs)?;
    ctx.f.emit(Op::JumpIfTrue);
    let jump_true = ctx.f.len();
    ctx.f.emit_i16(0);
    compile_expr(ctx, rhs)?;
    ctx.f.emit(Op::Jump);
    let jump_end = ctx.f.len();
    ctx.f.emit_i16(0);
    let true_at = ctx.f.len();
    ctx.f
        .patch_i16(jump_true, ((true_at as isize) - ((jump_true + 2) as isize)) as i16);
    ctx.f.emit(Op::LoadTrue);
    let end = ctx.f.len();
    ctx.f
        .patch_i16(jump_end, ((end as isize) - ((jump_end + 2) as isize)) as i16);
    Ok(())
}
