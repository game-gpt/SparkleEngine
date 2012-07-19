//! Oaks `oak-valkyrie` AST → `spark-vm` 字节码。
//!
//! 支持子集：`micro` / `let` / `return` / `if` / `loop while`、算术比较、
//! 调用（脚本 micro / 原生 / `print`）。完整语言其余构造报 `unsupported_*`。

use std::collections::{HashMap, HashSet};

use oak_valkyrie::ValkyrieTokenType;
use oak_valkyrie::ast::{
    Block, ExprStmt, Let, MicroDeclaration, Pattern, Statement, StatementNode, StringLiteral,
    StringSegment, TermExpression, ValkyrieRoot,
};
use spark_vm::{FuncProto, Module, Op};

pub(crate) fn compile_root(root: &ValkyrieRoot, native_names: &[&str]) -> Result<Module, String> {
    let native_set: HashSet<&str> = native_names.iter().copied().collect();
    let mut functions: Vec<FuncProto> = Vec::new();
    let mut fn_index: HashMap<String, usize> = HashMap::new();

    for item in &root.items {
        if let StatementNode::Micro(m) = item {
            let idx = functions.len();
            fn_index.insert(m.name.name.clone(), idx);
            functions.push(FuncProto::new(m.name.name.clone(), m.params.len() as u8));
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
        if let StatementNode::Micro(m) = item {
            let idx = *fn_index.get(&m.name.name).unwrap();
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
                StatementNode::Micro(_) => {}
                StatementNode::Let(l) => compile_let(&mut ctx, l)?,
                StatementNode::ExprStmt(s) => {
                    if matches!(s.expr, TermExpression::Return(_)) {
                        saw_return = true;
                    }
                    compile_expr_stmt(&mut ctx, s)?;
                }
                StatementNode::Statement(inner) => match inner.as_ref() {
                    StatementNode::Let(l) => compile_let(&mut ctx, l)?,
                    StatementNode::ExprStmt(s) => {
                        if matches!(s.expr, TermExpression::Return(_)) {
                            saw_return = true;
                        }
                        compile_expr_stmt(&mut ctx, s)?;
                    }
                    other => return Err(format!("unsupported_root_item:{other:?}")),
                },
                other => return Err(format!("unsupported_root_item:{other:?}")),
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
    m: &MicroDeclaration,
    fn_index: &HashMap<String, usize>,
    native_set: &HashSet<&str>,
) -> Result<(), String> {
    {
        let f = &mut module.functions[idx];
        f.locals = m.params.len() as u16;
    }
    let mut locals = HashMap::new();
    for (i, p) in m.params.iter().enumerate() {
        locals.insert(p.name.name.clone(), i as u16);
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

fn compile_block_as_body(ctx: &mut Ctx<'_>, body: &Block) -> Result<(), String> {
    if body.statements.is_empty() {
        ctx.f.emit(Op::LoadNull);
        ctx.f.emit(Op::Return);
        return Ok(());
    }
    let mut saw_return = false;
    for s in &body.statements {
        if stmt_is_return(s) {
            saw_return = true;
        }
        compile_statement(ctx, s)?;
    }
    if !saw_return {
        ctx.f.emit(Op::LoadNull);
        ctx.f.emit(Op::Return);
    }
    Ok(())
}

fn stmt_is_return(s: &Statement) -> bool {
    match s {
        Statement::ExprStmt(e) => matches!(e.expr, TermExpression::Return(_)),
        Statement::Let(_) => false,
    }
}

fn compile_statement(ctx: &mut Ctx<'_>, s: &Statement) -> Result<(), String> {
    match s {
        Statement::Let(l) => compile_let(ctx, l),
        Statement::ExprStmt(e) => compile_expr_stmt(ctx, e),
    }
}

fn compile_let(ctx: &mut Ctx<'_>, l: &Let) -> Result<(), String> {
    let name = match &l.pattern {
        Pattern::Variable(v) => v.name.name.clone(),
        other => return Err(format!("unsupported_let_pattern:{other:?}")),
    };
    compile_expr(ctx, &l.expr)?;
    let slot = ctx.alloc_local(&name);
    ctx.f.emit(Op::StoreLocal);
    ctx.f.emit_u16(slot);
    Ok(())
}

fn compile_expr_stmt(ctx: &mut Ctx<'_>, s: &ExprStmt) -> Result<(), String> {
    if matches!(s.expr, TermExpression::Return(_)) {
        return compile_expr(ctx, &s.expr);
    }
    compile_expr(ctx, &s.expr)?;
    ctx.f.emit(Op::Pop);
    ctx.f.emit_u8(1);
    Ok(())
}

fn compile_stmts(ctx: &mut Ctx<'_>, body: &Block) -> Result<(), String> {
    for s in &body.statements {
        compile_statement(ctx, s)?;
    }
    Ok(())
}

fn compile_expr(ctx: &mut Ctx<'_>, e: &TermExpression) -> Result<(), String> {
    match e {
        TermExpression::Bool { value: true, .. } => ctx.f.emit(Op::LoadTrue),
        TermExpression::Bool { value: false, .. } => ctx.f.emit(Op::LoadFalse),
        TermExpression::StringLiteral(lit) => compile_literal(ctx, lit)?,
        TermExpression::NamePath(path) => {
            if path.parts.len() != 1 {
                return Err(format!(
                    "unsupported_qualified_name:{}",
                    path.parts
                        .iter()
                        .map(|p| p.name.as_str())
                        .collect::<Vec<_>>()
                        .join("::")
                ));
            }
            let name = &path.parts[0].name;
            if name == "null" {
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
        TermExpression::Unary(u) => {
            compile_expr(ctx, &u.base)?;
            match u.operator {
                ValkyrieTokenType::Minus => ctx.f.emit(Op::Neg),
                ValkyrieTokenType::Bang => ctx.f.emit(Op::Not),
                other => return Err(format!("unsupported_unary:{other:?}")),
            }
        }
        TermExpression::Binary(b) => match b.operator {
            ValkyrieTokenType::AndAnd => compile_and(ctx, &b.lhs, &b.rhs)?,
            ValkyrieTokenType::OrOr => compile_or(ctx, &b.lhs, &b.rhs)?,
            other => {
                compile_expr(ctx, &b.lhs)?;
                compile_expr(ctx, &b.rhs)?;
                let opc = match other {
                    ValkyrieTokenType::Plus => Op::Add,
                    ValkyrieTokenType::Minus => Op::Sub,
                    ValkyrieTokenType::Star => Op::Mul,
                    ValkyrieTokenType::Slash => Op::Div,
                    ValkyrieTokenType::EqEq => Op::Eq,
                    ValkyrieTokenType::NotEq => Op::Ne,
                    ValkyrieTokenType::LessThan => Op::Lt,
                    ValkyrieTokenType::LessEq => Op::Le,
                    ValkyrieTokenType::GreaterThan => Op::Gt,
                    ValkyrieTokenType::GreaterEq => Op::Ge,
                    _ => return Err(format!("unsupported_binary:{other:?}")),
                };
                ctx.f.emit(opc);
            }
        },
        TermExpression::Paren { expr, .. } => compile_expr(ctx, expr)?,
        TermExpression::ApplyCall { callee, args, .. } => compile_call(ctx, callee, args)?,
        TermExpression::Return(r) => {
            match &r.base {
                Some(v) => compile_expr(ctx, v)?,
                None => ctx.f.emit(Op::LoadNull),
            }
            ctx.f.emit(Op::Return);
        }
        TermExpression::If {
            pattern,
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            if pattern.is_some() {
                return Err("unsupported_if_let".into());
            }
            compile_expr(ctx, condition)?;
            ctx.f.emit(Op::JumpIfFalse);
            let jf = ctx.f.len();
            ctx.f.emit_i16(0);
            compile_stmts(ctx, then_branch)?;
            ctx.f.emit(Op::LoadNull);
            ctx.f.emit(Op::Jump);
            let jend = ctx.f.len();
            ctx.f.emit_i16(0);
            let else_start = ctx.f.len();
            ctx.f
                .patch_i16(jf, ((else_start as isize) - ((jf + 2) as isize)) as i16);
            if let Some(eb) = else_branch {
                compile_stmts(ctx, eb)?;
            }
            ctx.f.emit(Op::LoadNull);
            let end = ctx.f.len();
            ctx.f
                .patch_i16(jend, ((end as isize) - ((jend + 2) as isize)) as i16);
        }
        TermExpression::Loop {
            condition,
            pattern,
            body,
            ..
        } => {
            if pattern.is_some() {
                return Err("unsupported_for_loop".into());
            }
            let Some(cond) = condition else {
                return Err("unsupported_infinite_loop".into());
            };
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
        TermExpression::Block(body) => {
            compile_stmts(ctx, body)?;
            ctx.f.emit(Op::LoadNull);
        }
        other => return Err(format!("unsupported_expr:{other:?}")),
    }
    Ok(())
}

/// Oaks 把数字字面量也建成 `StringLiteral{quote_count:0}`；带引号的才是字符串。
fn compile_literal(ctx: &mut Ctx<'_>, lit: &StringLiteral) -> Result<(), String> {
    let text = plain_text(lit)?;
    if lit.quote_count == 0 {
        if text == "null" {
            ctx.f.emit(Op::LoadNull);
            return Ok(());
        }
        let n: f64 = text
            .parse()
            .map_err(|_| format!("invalid_number:{text}"))?;
        let i = ctx.f.add_const_number(n);
        ctx.f.emit(Op::LoadConst);
        ctx.f.emit_u16(i);
        return Ok(());
    }
    let i = ctx.f.add_string(text);
    ctx.f.emit(Op::LoadString);
    ctx.f.emit_u16(i);
    Ok(())
}

fn plain_text(lit: &StringLiteral) -> Result<String, String> {
    let mut out = String::new();
    for seg in &lit.segments {
        match seg {
            StringSegment::Text(t) => out.push_str(&t.content),
            StringSegment::Interpolation(_) => {
                return Err("unsupported_string_interpolation".into());
            }
        }
    }
    Ok(out)
}

fn compile_call(
    ctx: &mut Ctx<'_>,
    callee: &TermExpression,
    args: &[TermExpression],
) -> Result<(), String> {
    let name = match callee {
        TermExpression::NamePath(path) if path.parts.len() == 1 => path.parts[0].name.clone(),
        other => return Err(format!("unsupported_callee:{other:?}")),
    };
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
        let ni = ctx.intern_native(&name);
        ctx.f.emit(Op::CallNative);
        ctx.f.emit_u16(ni);
        ctx.f.emit_u8(args.len() as u8);
        return Ok(());
    }
    if let Some(&fidx) = ctx.fn_index.get(&name) {
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
    Err(format!("unknown_function:{name}"))
}

fn compile_and(
    ctx: &mut Ctx<'_>,
    lhs: &TermExpression,
    rhs: &TermExpression,
) -> Result<(), String> {
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

fn compile_or(
    ctx: &mut Ctx<'_>,
    lhs: &TermExpression,
    rhs: &TermExpression,
) -> Result<(), String> {
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
