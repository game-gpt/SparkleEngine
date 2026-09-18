//! 手写 Valkyrie 子集 AST → `spark-vm` 字节码。

use std::collections::{HashMap, HashSet};

use spark_vm::{FuncProto, Module, Op};

use crate::ast::{BinOp, Expr, Item, Micro, Stmt, UnaryOp, ValkyrieRoot};

pub fn compile_root(root: &ValkyrieRoot, native_names: &[&str]) -> Result<Module, String> {
    let native_set: HashSet<&str> = native_names.iter().copied().collect();
    let mut functions: Vec<FuncProto> = Vec::new();
    let mut fn_index: HashMap<String, usize> = HashMap::new();

    for item in &root.items {
        if let Item::Micro(m) = item {
            let idx = functions.len();
            fn_index.insert(m.name.clone(), idx);
            functions.push(FuncProto::new(m.name.clone(), m.params.len() as u8));
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

    for item in &root.items {
        if let Item::Micro(m) = item {
            let idx = *fn_index.get(&m.name).unwrap();
            compile_micro(&mut module, idx, m, &fn_index, &native_set)?;
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
        for item in &root.items {
            match item {
                Item::Micro(_) => {}
                Item::Stmt(s) => {
                    if matches!(s, Stmt::Return(_)) {
                        saw_return = true;
                    }
                    compile_stmt(&mut ctx, s)?;
                }
            }
        }
        if !saw_return {
            ctx.f.emit(Op::LoadNull);
            ctx.f.emit(Op::Return);
        }
    }

    Ok(module)
}

fn compile_micro(
    module: &mut Module,
    idx: usize,
    m: &Micro,
    fn_index: &HashMap<String, usize>,
    native_set: &HashSet<&str>,
) -> Result<(), String> {
    {
        let f = &mut module.functions[idx];
        f.locals = m.params.len() as u16;
    }
    let mut locals = HashMap::new();
    for (i, p) in m.params.iter().enumerate() {
        locals.insert(p.clone(), i as u16);
    }
    let mut ctx = Ctx {
        f: &mut module.functions[idx],
        locals,
        fn_index,
        native_set,
        module_natives: &mut module.native_names,
    };
    compile_block_as_body(&mut ctx, &m.body)?;
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

fn compile_block_as_body(ctx: &mut Ctx<'_>, body: &[Stmt]) -> Result<(), String> {
    if body.is_empty() {
        ctx.f.emit(Op::LoadNull);
        ctx.f.emit(Op::Return);
        return Ok(());
    }
    let mut saw_return = false;
    for s in body {
        if matches!(s, Stmt::Return(_)) {
            saw_return = true;
        }
        compile_stmt(ctx, s)?;
    }
    if !saw_return {
        ctx.f.emit(Op::LoadNull);
        ctx.f.emit(Op::Return);
    }
    Ok(())
}

fn compile_stmt(ctx: &mut Ctx<'_>, s: &Stmt) -> Result<(), String> {
    match s {
        Stmt::Let { name, value } => {
            compile_expr(ctx, value)?;
            let slot = ctx.alloc_local(name);
            ctx.f.emit(Op::StoreLocal);
            ctx.f.emit_u16(slot);
            Ok(())
        }
        Stmt::Expr(e) => {
            compile_expr(ctx, e)?;
            // `Print` 不弹出栈顶，表达式语句统一丢弃结果。
            ctx.f.emit(Op::Pop);
            ctx.f.emit_u8(1);
            Ok(())
        }
        Stmt::Return(v) => {
            match v {
                Some(e) => compile_expr(ctx, e)?,
                None => ctx.f.emit(Op::LoadNull),
            }
            ctx.f.emit(Op::Return);
            Ok(())
        }
    }
}

fn compile_stmts(ctx: &mut Ctx<'_>, body: &[Stmt]) -> Result<(), String> {
    for s in body {
        compile_stmt(ctx, s)?;
    }
    Ok(())
}

fn compile_expr(ctx: &mut Ctx<'_>, e: &Expr) -> Result<(), String> {
    match e {
        Expr::Null => ctx.f.emit(Op::LoadNull),
        Expr::Bool(true) => ctx.f.emit(Op::LoadTrue),
        Expr::Bool(false) => ctx.f.emit(Op::LoadFalse),
        Expr::Number(n) => {
            let i = ctx.f.add_const_number(*n);
            ctx.f.emit(Op::LoadConst);
            ctx.f.emit_u16(i);
        }
        Expr::String(s) => {
            let i = ctx.f.add_string(s.clone());
            ctx.f.emit(Op::LoadString);
            ctx.f.emit_u16(i);
        }
        Expr::Name(name) => {
            if let Some(&slot) = ctx.locals.get(name) {
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
        Expr::Unary { op, expr } => {
            compile_expr(ctx, expr)?;
            match op {
                UnaryOp::Neg => ctx.f.emit(Op::Neg),
                UnaryOp::Not => ctx.f.emit(Op::Not),
            }
        }
        Expr::Binary { op, lhs, rhs } => match op {
            BinOp::And => compile_and(ctx, lhs, rhs)?,
            BinOp::Or => compile_or(ctx, lhs, rhs)?,
            other => {
                compile_expr(ctx, lhs)?;
                compile_expr(ctx, rhs)?;
                let opc = match other {
                    BinOp::Add => Op::Add,
                    BinOp::Sub => Op::Sub,
                    BinOp::Mul => Op::Mul,
                    BinOp::Div => Op::Div,
                    BinOp::Eq => Op::Eq,
                    BinOp::Ne => Op::Ne,
                    BinOp::Lt => Op::Lt,
                    BinOp::Le => Op::Le,
                    BinOp::Gt => Op::Gt,
                    BinOp::Ge => Op::Ge,
                    BinOp::And | BinOp::Or => unreachable!(),
                };
                ctx.f.emit(opc);
            }
        },
        Expr::Call { name, args } => {
            if name == "print" || name == "puts" {
                if args.len() != 1 {
                    return Err("print_arity_one".into());
                }
                compile_expr(ctx, &args[0])?;
                ctx.f.emit(Op::Print);
                return Ok(());
            }
            if ctx.native_set.contains(name.as_str()) {
                for a in args {
                    compile_expr(ctx, a)?;
                }
                let ni = ctx.intern_native(name);
                ctx.f.emit(Op::CallNative);
                ctx.f.emit_u16(ni);
                ctx.f.emit_u8(args.len() as u8);
                return Ok(());
            }
            if let Some(&fidx) = ctx.fn_index.get(name) {
                let i = ctx.f.add_const_func(fidx as u32);
                ctx.f.emit(Op::LoadConst);
                ctx.f.emit_u16(i);
                for a in args {
                    compile_expr(ctx, a)?;
                }
                ctx.f.emit(Op::Call);
                ctx.f.emit_u8(args.len() as u8);
                return Ok(());
            }
            return Err(format!("unknown_function:{name}"));
        }
        Expr::If {
            cond,
            then_body,
            else_body,
        } => {
            compile_expr(ctx, cond)?;
            ctx.f.emit(Op::JumpIfFalse);
            let jf = ctx.f.len();
            ctx.f.emit_i16(0);
            compile_stmts(ctx, then_body)?;
            ctx.f.emit(Op::LoadNull);
            ctx.f.emit(Op::Jump);
            let jend = ctx.f.len();
            ctx.f.emit_i16(0);
            let else_start = ctx.f.len();
            ctx.f
                .patch_i16(jf, ((else_start as isize) - ((jf + 2) as isize)) as i16);
            if let Some(eb) = else_body {
                compile_stmts(ctx, eb)?;
            }
            ctx.f.emit(Op::LoadNull);
            let end = ctx.f.len();
            ctx.f
                .patch_i16(jend, ((end as isize) - ((jend + 2) as isize)) as i16);
        }
        Expr::While { cond, body } => {
            let loop_start = ctx.f.len();
            compile_expr(ctx, cond)?;
            ctx.f.emit(Op::JumpIfFalse);
            let jf = ctx.f.len();
            ctx.f.emit_i16(0);
            compile_stmts(ctx, body)?;
            ctx.f.emit(Op::Jump);
            let back = ctx.f.len();
            ctx.f.emit_i16(0);
            let end = ctx.f.len();
            ctx.f
                .patch_i16(jf, ((end as isize) - ((jf + 2) as isize)) as i16);
            ctx.f
                .patch_i16(back, ((loop_start as isize) - ((back + 2) as isize)) as i16);
            ctx.f.emit(Op::LoadNull);
        }
        Expr::Block(body) => {
            compile_stmts(ctx, body)?;
            ctx.f.emit(Op::LoadNull);
        }
    }
    Ok(())
}

fn compile_and(ctx: &mut Ctx<'_>, lhs: &Expr, rhs: &Expr) -> Result<(), String> {
    compile_expr(ctx, lhs)?;
    ctx.f.emit(Op::JumpIfFalse);
    let jf = ctx.f.len();
    ctx.f.emit_i16(0);
    compile_expr(ctx, rhs)?;
    ctx.f.emit(Op::Jump);
    let jend = ctx.f.len();
    ctx.f.emit_i16(0);
    let false_at = ctx.f.len();
    ctx.f
        .patch_i16(jf, ((false_at as isize) - ((jf + 2) as isize)) as i16);
    ctx.f.emit(Op::LoadFalse);
    let end = ctx.f.len();
    ctx.f
        .patch_i16(jend, ((end as isize) - ((jend + 2) as isize)) as i16);
    Ok(())
}

fn compile_or(ctx: &mut Ctx<'_>, lhs: &Expr, rhs: &Expr) -> Result<(), String> {
    compile_expr(ctx, lhs)?;
    ctx.f.emit(Op::JumpIfTrue);
    let jt = ctx.f.len();
    ctx.f.emit_i16(0);
    compile_expr(ctx, rhs)?;
    ctx.f.emit(Op::Jump);
    let jend = ctx.f.len();
    ctx.f.emit_i16(0);
    let true_at = ctx.f.len();
    ctx.f
        .patch_i16(jt, ((true_at as isize) - ((jt + 2) as isize)) as i16);
    ctx.f.emit(Op::LoadTrue);
    let end = ctx.f.len();
    ctx.f
        .patch_i16(jend, ((end as isize) - ((jend + 2) as isize)) as i16);
    Ok(())
}
