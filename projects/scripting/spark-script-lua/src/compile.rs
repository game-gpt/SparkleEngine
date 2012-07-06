//! Oaks `oak-lua` AST → `spark-vm` 字节码。
//!
//! 支持子集：`function` / `local`、`return`、`if`/`while`、算术比较、
//! 调用（脚本函数 / 原生 / `print`）。不支持 table / 元表 / 泛型 for / goto。

use std::collections::{HashMap, HashSet};

use oak_lua::ast::{
    LuaAssignmentStatement, LuaCallExpression, LuaExpression, LuaFunctionStatement, LuaIfStatement,
    LuaLocalStatement, LuaRoot, LuaStatement, LuaWhileStatement,
};
use spark_vm::{FuncProto, Module, Op};

pub(crate) fn compile_root(root: &LuaRoot, native_names: &[&str]) -> Result<Module, String> {
    let native_set: HashSet<&str> = native_names.iter().copied().collect();
    let mut functions: Vec<FuncProto> = Vec::new();
    let mut fn_index: HashMap<String, usize> = HashMap::new();

    for stmt in &root.statements {
        if let LuaStatement::Function(f) = stmt {
            let name = function_name(f)?;
            let idx = functions.len();
            fn_index.insert(name.clone(), idx);
            functions.push(FuncProto::new(name, f.parameters.len() as u8));
        }
    }

    let main_idx = functions.len();
    functions.push(FuncProto::new("__main", 0));

    let mut module = Module {
        functions,
        entry: main_idx,
        native_names: Vec::new(),
    };
    for n in native_names {
        module.intern_native(*n);
    }

    for stmt in &root.statements {
        if let LuaStatement::Function(f) = stmt {
            let name = function_name(f)?;
            let idx = *fn_index.get(&name).unwrap();
            compile_function(&mut module, idx, f, &fn_index, &native_set)?;
        }
    }

    {
        let mut ctx = Ctx {
            f: &mut module.functions[main_idx],
            locals: HashMap::new(),
            fn_index: &fn_index,
            native_set: &native_set,
            module_natives: &mut module.native_names,
        };
        let mut saw_return = false;
        for stmt in &root.statements {
            match stmt {
                LuaStatement::Function(_) => {}
                LuaStatement::Return(_) => {
                    saw_return = true;
                    compile_statement(&mut ctx, stmt)?;
                }
                other => compile_statement(&mut ctx, other)?,
            }
        }
        if !saw_return {
            ctx.f.emit(Op::LoadNull);
            ctx.f.emit(Op::Return);
        }
    }

    Ok(module)
}

fn function_name(f: &LuaFunctionStatement) -> Result<String, String> {
    if f.receiver.is_some() {
        return Err("unsupported_method_def".into());
    }
    if f.name.len() != 1 {
        return Err(format!(
            "unsupported_qualified_name:{}",
            f.name.join(".")
        ));
    }
    if f.is_vararg {
        return Err("unsupported_varargs".into());
    }
    Ok(f.name[0].clone())
}

fn compile_function(
    module: &mut Module,
    idx: usize,
    f: &LuaFunctionStatement,
    fn_index: &HashMap<String, usize>,
    native_set: &HashSet<&str>,
) -> Result<(), String> {
    {
        let proto = &mut module.functions[idx];
        proto.locals = f.parameters.len() as u16;
    }
    let mut locals = HashMap::new();
    for (i, p) in f.parameters.iter().enumerate() {
        locals.insert(p.clone(), i as u16);
    }
    let mut ctx = Ctx {
        f: &mut module.functions[idx],
        locals,
        fn_index,
        native_set,
        module_natives: &mut module.native_names,
    };
    compile_block_as_body(&mut ctx, &f.block)?;
    Ok(())
}

struct Ctx<'a> {
    f: &'a mut FuncProto,
    locals: HashMap<String, u16>,
    fn_index: &'a HashMap<String, usize>,
    native_set: &'a HashSet<&'a str>,
    module_natives: &'a mut Vec<String>,
}

impl Ctx<'_> {
    fn alloc_local(&mut self, name: &str) -> u16 {
        if let Some(&i) = self.locals.get(name) {
            return i;
        }
        let i = self.locals.len() as u16;
        self.locals.insert(name.to_string(), i);
        self.f.locals = self.f.locals.max(i + 1);
        i
    }

    fn intern_native(&mut self, name: &str) -> u16 {
        if let Some(i) = self.module_natives.iter().position(|n| n == name) {
            return i as u16;
        }
        let i = self.module_natives.len() as u16;
        self.module_natives.push(name.to_string());
        i
    }
}

fn compile_block_as_body(ctx: &mut Ctx<'_>, block: &[LuaStatement]) -> Result<(), String> {
    if block.is_empty() {
        ctx.f.emit(Op::LoadNull);
        ctx.f.emit(Op::Return);
        return Ok(());
    }
    let mut saw_return = false;
    for stmt in block {
        if matches!(stmt, LuaStatement::Return(_)) {
            saw_return = true;
        }
        compile_statement(ctx, stmt)?;
    }
    if !saw_return {
        ctx.f.emit(Op::LoadNull);
        ctx.f.emit(Op::Return);
    }
    Ok(())
}

fn compile_statement(ctx: &mut Ctx<'_>, stmt: &LuaStatement) -> Result<(), String> {
    match stmt {
        LuaStatement::Local(l) => compile_local(ctx, l),
        LuaStatement::Assignment(a) => compile_assignment(ctx, a),
        LuaStatement::Expression(e) => {
            compile_expr(ctx, e)?;
            ctx.f.emit(Op::Pop);
            ctx.f.emit_u8(1);
            Ok(())
        }
        LuaStatement::Return(r) => {
            match r.values.as_slice() {
                [] => ctx.f.emit(Op::LoadNull),
                [v] => compile_expr(ctx, v)?,
                _ => return Err("single_return_only".into()),
            }
            ctx.f.emit(Op::Return);
            Ok(())
        }
        LuaStatement::If(i) => compile_if(ctx, i),
        LuaStatement::While(w) => compile_while(ctx, w),
        LuaStatement::Function(_) => Err("nested_function_unsupported".into()),
        LuaStatement::Do(block) => {
            for s in block {
                compile_statement(ctx, s)?;
            }
            Ok(())
        }
        other => Err(format!("unsupported_stmt:{other:?}")),
    }
}

fn compile_local(ctx: &mut Ctx<'_>, l: &LuaLocalStatement) -> Result<(), String> {
    if l.names.len() != 1 {
        return Err("single_local_binding_only".into());
    }
    let name = &l.names[0];
    match l.values.as_slice() {
        [] => ctx.f.emit(Op::LoadNull),
        [v] => compile_expr(ctx, v)?,
        _ => return Err("single_local_init_only".into()),
    }
    let slot = ctx.alloc_local(name);
    ctx.f.emit(Op::StoreLocal);
    ctx.f.emit_u16(slot);
    Ok(())
}

fn compile_assignment(ctx: &mut Ctx<'_>, a: &LuaAssignmentStatement) -> Result<(), String> {
    if a.targets.len() != 1 || a.values.len() != 1 {
        return Err("single_assign_target_only".into());
    }
    let LuaExpression::Identifier(name) = &a.targets[0] else {
        return Err("ident_assign_only".into());
    };
    compile_expr(ctx, &a.values[0])?;
    let slot = ctx.alloc_local(name);
    ctx.f.emit(Op::StoreLocal);
    ctx.f.emit_u16(slot);
    Ok(())
}

fn compile_if(ctx: &mut Ctx<'_>, i: &LuaIfStatement) -> Result<(), String> {
    if !i.else_ifs.is_empty() {
        return Err("unsupported_elseif".into());
    }
    compile_expr(ctx, &i.condition)?;
    ctx.f.emit(Op::JumpIfFalse);
    let jf = ctx.f.len();
    ctx.f.emit_i16(0);
    for s in &i.then_block {
        compile_statement(ctx, s)?;
    }
    ctx.f.emit(Op::Jump);
    let jend = ctx.f.len();
    ctx.f.emit_i16(0);
    let else_start = ctx.f.len();
    ctx.f.patch_i16(jf, ((else_start as isize) - ((jf + 2) as isize)) as i16);
    if let Some(eb) = &i.else_block {
        for s in eb {
            compile_statement(ctx, s)?;
        }
    }
    let end = ctx.f.len();
    ctx.f.patch_i16(jend, ((end as isize) - ((jend + 2) as isize)) as i16);
    Ok(())
}

fn compile_while(ctx: &mut Ctx<'_>, w: &LuaWhileStatement) -> Result<(), String> {
    let loop_start = ctx.f.len();
    compile_expr(ctx, &w.condition)?;
    ctx.f.emit(Op::JumpIfFalse);
    let jf = ctx.f.len();
    ctx.f.emit_i16(0);
    for s in &w.block {
        compile_statement(ctx, s)?;
    }
    ctx.f.emit(Op::Jump);
    let back = ctx.f.len();
    ctx.f.emit_i16(0);
    let end = ctx.f.len();
    ctx.f.patch_i16(jf, ((end as isize) - ((jf + 2) as isize)) as i16);
    ctx.f.patch_i16(back, ((loop_start as isize) - ((back + 2) as isize)) as i16);
    Ok(())
}

fn compile_expr(ctx: &mut Ctx<'_>, expr: &LuaExpression) -> Result<(), String> {
    match expr {
        LuaExpression::Nil => ctx.f.emit(Op::LoadNull),
        LuaExpression::Boolean(true) => ctx.f.emit(Op::LoadTrue),
        LuaExpression::Boolean(false) => ctx.f.emit(Op::LoadFalse),
        LuaExpression::Number(n) => {
            let i = ctx.f.add_const_number(*n);
            ctx.f.emit(Op::LoadConst);
            ctx.f.emit_u16(i);
        }
        LuaExpression::String(s) => {
            let i = ctx.f.add_string(s.clone());
            ctx.f.emit(Op::LoadString);
            ctx.f.emit_u16(i);
        }
        LuaExpression::Identifier(name) => {
            if name == "nil" {
                ctx.f.emit(Op::LoadNull);
            } else if let Some(&slot) = ctx.locals.get(name) {
                ctx.f.emit(Op::LoadLocal);
                ctx.f.emit_u16(slot);
            } else if let Some(&fidx) = ctx.fn_index.get(name) {
                let i = ctx.f.add_const_func(fidx as u32);
                ctx.f.emit(Op::LoadConst);
                ctx.f.emit_u16(i);
            } else {
                let g = ctx.f.add_const_name(name.clone());
                ctx.f.emit(Op::LoadGlobal);
                ctx.f.emit_u16(g);
            }
        }
        LuaExpression::Unary(u) => {
            compile_expr(ctx, &u.operand)?;
            match u.op.as_str() {
                "-" => ctx.f.emit(Op::Neg),
                "not" => ctx.f.emit(Op::Not),
                other => return Err(format!("unsupported_unary:{other}")),
            }
        }
        LuaExpression::Binary(b) => match b.op.as_str() {
            "and" => compile_and(ctx, &b.left, &b.right)?,
            "or" => compile_or(ctx, &b.left, &b.right)?,
            other => {
                compile_expr(ctx, &b.left)?;
                compile_expr(ctx, &b.right)?;
                let op = match other {
                    "+" => Op::Add,
                    "-" => Op::Sub,
                    "*" => Op::Mul,
                    "/" => Op::Div,
                    "==" => Op::Eq,
                    "~=" | "!=" => Op::Ne,
                    "<" => Op::Lt,
                    "<=" => Op::Le,
                    ">" => Op::Gt,
                    ">=" => Op::Ge,
                    _ => return Err(format!("unsupported_binary:{other}")),
                };
                ctx.f.emit(op);
            }
        },
        LuaExpression::Call(c) => compile_call(ctx, c)?,
        other => return Err(format!("unsupported_expr:{other:?}")),
    }
    Ok(())
}

fn compile_call(ctx: &mut Ctx<'_>, c: &LuaCallExpression) -> Result<(), String> {
    if let LuaExpression::Identifier(name) = &c.function {
        if name == "print" || name == "println" {
            if c.arguments.len() != 1 {
                return Err("print_arity_one".into());
            }
            compile_expr(ctx, &c.arguments[0])?;
            ctx.f.emit(Op::Print);
            return Ok(());
        }
        if ctx.native_set.contains(name.as_str()) {
            for a in &c.arguments {
                compile_expr(ctx, a)?;
            }
            let ni = ctx.intern_native(name);
            ctx.f.emit(Op::CallNative);
            ctx.f.emit_u16(ni);
            ctx.f.emit_u8(c.arguments.len() as u8);
            return Ok(());
        }
        if let Some(&fidx) = ctx.fn_index.get(name) {
            let i = ctx.f.add_const_func(fidx as u32);
            ctx.f.emit(Op::LoadConst);
            ctx.f.emit_u16(i);
            for a in &c.arguments {
                compile_expr(ctx, a)?;
            }
            ctx.f.emit(Op::Call);
            ctx.f.emit_u8(c.arguments.len() as u8);
            return Ok(());
        }
    }
    compile_expr(ctx, &c.function)?;
    for a in &c.arguments {
        compile_expr(ctx, a)?;
    }
    ctx.f.emit(Op::Call);
    ctx.f.emit_u8(c.arguments.len() as u8);
    Ok(())
}

fn compile_and(ctx: &mut Ctx<'_>, lhs: &LuaExpression, rhs: &LuaExpression) -> Result<(), String> {
    compile_expr(ctx, lhs)?;
    ctx.f.emit(Op::JumpIfFalse);
    let jf = ctx.f.len();
    ctx.f.emit_i16(0);
    compile_expr(ctx, rhs)?;
    ctx.f.emit(Op::Jump);
    let jend = ctx.f.len();
    ctx.f.emit_i16(0);
    let false_at = ctx.f.len();
    ctx.f.patch_i16(jf, ((false_at as isize) - ((jf + 2) as isize)) as i16);
    ctx.f.emit(Op::LoadFalse);
    let end = ctx.f.len();
    ctx.f.patch_i16(jend, ((end as isize) - ((jend + 2) as isize)) as i16);
    Ok(())
}

fn compile_or(ctx: &mut Ctx<'_>, lhs: &LuaExpression, rhs: &LuaExpression) -> Result<(), String> {
    compile_expr(ctx, lhs)?;
    ctx.f.emit(Op::JumpIfTrue);
    let jt = ctx.f.len();
    ctx.f.emit_i16(0);
    compile_expr(ctx, rhs)?;
    ctx.f.emit(Op::Jump);
    let jend = ctx.f.len();
    ctx.f.emit_i16(0);
    let true_at = ctx.f.len();
    ctx.f.patch_i16(jt, ((true_at as isize) - ((jt + 2) as isize)) as i16);
    ctx.f.emit(Op::LoadTrue);
    let end = ctx.f.len();
    ctx.f.patch_i16(jend, ((end as isize) - ((jend + 2) as isize)) as i16);
    Ok(())
}
