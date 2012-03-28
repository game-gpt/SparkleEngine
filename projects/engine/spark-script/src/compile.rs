//! AST → `spark-vm` 字节码。

use std::collections::HashMap;

use spark_gc::Value;
use spark_vm::{FuncProto, Module, Op};

use crate::lex::TokenKind;
use crate::parse::{Expr, Program, Stmt};

pub fn compile_program(program: &Program) -> Result<Module, String> {
    compile_program_with_natives(program, &[])
}

/// 编译程序；`native_names` 中的标识符走 `CallNative` 而非脚本函数。
pub fn compile_program_with_natives(
    program: &Program,
    native_names: &[&str],
) -> Result<Module, String> {
    let native_set: std::collections::HashSet<&str> = native_names.iter().copied().collect();
    let mut functions: Vec<FuncProto> = Vec::new();
    // 预扫描顶层 fn，占位
    let mut fn_index: HashMap<String, usize> = HashMap::new();
    for s in &program.stmts {
        if let Stmt::FnDef { name, params, .. } = s {
            let idx = functions.len();
            fn_index.insert(name.clone(), idx);
            functions.push(FuncProto::new(name.clone(), params.len() as u8));
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

    // 编译各函数体
    for s in &program.stmts {
        if let Stmt::FnDef {
            name,
            params,
            body,
        } = s
        {
            let idx = *fn_index.get(name).unwrap();
            let mut ctx = Ctx {
                f: &mut module.functions[idx],
                locals: HashMap::new(),
                fn_index: &fn_index,
                native_set: &native_set,
                module_natives: &mut module.native_names,
            };
            for (i, p) in params.iter().enumerate() {
                ctx.locals.insert(p.clone(), i as u16);
            }
            ctx.f.locals = params.len() as u16;
            for st in body {
                compile_stmt(&mut ctx, st)?;
            }
            // 隐式 return null
            ctx.f.emit(Op::LoadNull);
            ctx.f.emit(Op::Return);
        }
    }

    // 主程序：非 fn 语句
    {
        let mut ctx = Ctx {
            f: &mut module.functions[main_idx],
            locals: HashMap::new(),
            fn_index: &fn_index,
            native_set: &native_set,
            module_natives: &mut module.native_names,
        };
        let mut saw_return = false;
        for s in &program.stmts {
            if matches!(s, Stmt::FnDef { .. }) {
                continue;
            }
            if matches!(s, Stmt::Return(_)) {
                saw_return = true;
            }
            compile_stmt(&mut ctx, s)?;
        }
        if !saw_return {
            ctx.f.emit(Op::LoadNull);
            ctx.f.emit(Op::Return);
        }
    }

    Ok(module)
}

struct Ctx<'a> {
    f: &'a mut FuncProto,
    locals: HashMap<String, u16>,
    fn_index: &'a HashMap<String, usize>,
    native_set: &'a std::collections::HashSet<&'a str>,
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

fn compile_stmt(ctx: &mut Ctx<'_>, stmt: &Stmt) -> Result<(), String> {
    match stmt {
        Stmt::Let { name, init } => {
            compile_expr(ctx, init)?;
            let slot = ctx.alloc_local(name);
            ctx.f.emit(Op::StoreLocal);
            ctx.f.emit_u16(slot);
        }
        Stmt::Assign { name, value } => {
            compile_expr(ctx, value)?;
            if let Some(&slot) = ctx.locals.get(name) {
                ctx.f.emit(Op::StoreLocal);
                ctx.f.emit_u16(slot);
            } else {
                let g = ctx.f.add_const_name(name.clone());
                ctx.f.emit(Op::StoreGlobal);
                ctx.f.emit_u16(g);
            }
        }
        Stmt::Expr(e) => {
            compile_expr(ctx, e)?;
            ctx.f.emit(Op::Pop);
            ctx.f.emit_u8(1);
        }
        Stmt::Return(e) => {
            if let Some(e) = e {
                compile_expr(ctx, e)?;
            } else {
                ctx.f.emit(Op::LoadNull);
            }
            ctx.f.emit(Op::Return);
        }
        Stmt::FnDef { .. } => {}
        Stmt::Block(body) => {
            for s in body {
                compile_stmt(ctx, s)?;
            }
        }
        Stmt::If {
            cond,
            then_body,
            else_body,
        } => {
            compile_expr(ctx, cond)?;
            ctx.f.emit(Op::JumpIfFalse);
            let jf = ctx.f.len();
            ctx.f.emit_i16(0);
            for s in then_body {
                compile_stmt(ctx, s)?;
            }
            ctx.f.emit(Op::Jump);
            let jend = ctx.f.len();
            ctx.f.emit_i16(0);
            let else_start = ctx.f.len();
            let rel = (else_start as isize) - ((jf + 2) as isize);
            ctx.f.patch_i16(jf, rel as i16);
            for s in else_body {
                compile_stmt(ctx, s)?;
            }
            let end = ctx.f.len();
            let rel2 = (end as isize) - ((jend + 2) as isize);
            ctx.f.patch_i16(jend, rel2 as i16);
        }
        Stmt::While { cond, body } => {
            let loop_start = ctx.f.len();
            compile_expr(ctx, cond)?;
            ctx.f.emit(Op::JumpIfFalse);
            let jf = ctx.f.len();
            ctx.f.emit_i16(0);
            for s in body {
                compile_stmt(ctx, s)?;
            }
            ctx.f.emit(Op::Jump);
            let back = ctx.f.len();
            ctx.f.emit_i16(0);
            let end = ctx.f.len();
            let rel_exit = (end as isize) - ((jf + 2) as isize);
            ctx.f.patch_i16(jf, rel_exit as i16);
            let rel_back = (loop_start as isize) - ((back + 2) as isize);
            ctx.f.patch_i16(back, rel_back as i16);
        }
    }
    Ok(())
}

fn compile_expr(ctx: &mut Ctx<'_>, expr: &Expr) -> Result<(), String> {
    match expr {
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
        Expr::Ident(name) => {
            if let Some(&slot) = ctx.locals.get(name) {
                ctx.f.emit(Op::LoadLocal);
                ctx.f.emit_u16(slot);
            } else if let Some(&fidx) = ctx.fn_index.get(name) {
                let i = ctx.f.add_const_number(fidx as f64);
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
                TokenKind::Minus => ctx.f.emit(Op::Neg),
                TokenKind::Bang => ctx.f.emit(Op::Not),
                _ => return Err(format!("无效一元算符 {op:?}")),
            }
        }
        Expr::Binary { op, left, right } => {
            compile_expr(ctx, left)?;
            compile_expr(ctx, right)?;
            let opc = match op {
                TokenKind::Plus => Op::Add,
                TokenKind::Minus => Op::Sub,
                TokenKind::Star => Op::Mul,
                TokenKind::Slash => Op::Div,
                TokenKind::EqEq => Op::Eq,
                TokenKind::BangEq => Op::Ne,
                TokenKind::Lt => Op::Lt,
                TokenKind::Le => Op::Le,
                TokenKind::Gt => Op::Gt,
                TokenKind::Ge => Op::Ge,
                _ => return Err(format!("无效二元算符 {op:?}")),
            };
            ctx.f.emit(opc);
        }
        Expr::Call { callee, args } => {
            if self_is_native(ctx, callee) {
                for a in args {
                    compile_expr(ctx, a)?;
                }
                let ni = ctx.intern_native(callee);
                ctx.f.emit(Op::CallNative);
                ctx.f.emit_u16(ni);
                ctx.f.emit_u8(args.len() as u8);
            } else {
                let fidx = ctx
                    .fn_index
                    .get(callee)
                    .copied()
                    .ok_or_else(|| format!("未知函数 {callee}"))?;
                let i = ctx.f.add_const_number(fidx as f64);
                ctx.f.emit(Op::LoadConst);
                ctx.f.emit_u16(i);
                for a in args {
                    compile_expr(ctx, a)?;
                }
                ctx.f.emit(Op::Call);
                ctx.f.emit_u8(args.len() as u8);
            }
        }
        Expr::Print(e) => {
            compile_expr(ctx, e)?;
            ctx.f.emit(Op::Print);
        }
    }
    Ok(())
}

// 静默未用
#[allow(dead_code)]
fn _value_null() -> Value {
    Value::Null
}

fn self_is_native(ctx: &Ctx<'_>, name: &str) -> bool {
    ctx.native_set.contains(name)
}
