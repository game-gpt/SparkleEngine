//! Oaks `oak-valkyrie` AST → `spark-vm` 字节码。
//!
//! 支持子集：`micro`、顶层 `let` / 表达式、`return`、`if`、`loop while`、
//! 算术比较、调用（脚本函数 / 原生 / `print`）。数字字面量按 Valkyrie builder
//! 现状识别为 `quote_count == 0` 的 [`StringLiteral`]。

use std::collections::{HashMap, HashSet};

use oak_valkyrie::ast::{
    Block, ExprStmt, Let, MicroDeclaration, Pattern, Statement, StatementNode, StringLiteral,
    StringSegment, TermExpression, ValkyrieRoot,
};
use oak_valkyrie::ValkyrieTokenType;
use spark_vm::{FuncProto, Module, Op};

pub fn compile_root(root: &ValkyrieRoot, native_names: &[&str]) -> Result<Module, String> {
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
                StatementNode::Let(l) => {
                    compile_let(&mut ctx, l)?;
                }
                StatementNode::ExprStmt(e) => {
                    if matches!(e.expr, TermExpression::Return(_)) {
                        saw_return = true;
                    }
                    compile_expr_stmt(&mut ctx, e, /*keep_tail*/ false)?;
                }
                other => {
                    return Err(format!("暂不支持的顶层项：{other:?}"));
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
    m: &MicroDeclaration,
    fn_index: &HashMap<String, usize>,
    native_set: &HashSet<&str>,
) -> Result<(), String> {
    // 拆开借用：先写 locals 元数据再编译体
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

fn compile_block_as_body(ctx: &mut Ctx<'_>, block: &Block) -> Result<(), String> {
    let n = block.statements.len();
    if n == 0 {
        ctx.f.emit(Op::LoadNull);
        ctx.f.emit(Op::Return);
        return Ok(());
    }
    for (i, stmt) in block.statements.iter().enumerate() {
        let is_last = i + 1 == n;
        match stmt {
            Statement::Let(l) => {
                compile_let(ctx, l)?;
                if is_last {
                    ctx.f.emit(Op::LoadNull);
                    ctx.f.emit(Op::Return);
                }
            }
            Statement::ExprStmt(e) => {
                if is_last {
                    // 末尾表达式作为返回值（无分号）；`return` 自带 Return。
                    if matches!(e.expr, TermExpression::Return(_)) {
                        compile_expr(ctx, &e.expr)?;
                    } else if e.semi {
                        compile_expr(ctx, &e.expr)?;
                        ctx.f.emit(Op::Pop);
                        ctx.f.emit_u8(1);
                        ctx.f.emit(Op::LoadNull);
                        ctx.f.emit(Op::Return);
                    } else {
                        compile_expr(ctx, &e.expr)?;
                        ctx.f.emit(Op::Return);
                    }
                } else {
                    compile_expr_stmt(ctx, e, false)?;
                }
            }
        }
    }
    Ok(())
}

fn compile_let(ctx: &mut Ctx<'_>, l: &Let) -> Result<(), String> {
    compile_expr(ctx, &l.expr)?;
    let name = pattern_name(&l.pattern)?;
    let slot = ctx.alloc_local(&name);
    ctx.f.emit(Op::StoreLocal);
    ctx.f.emit_u16(slot);
    Ok(())
}

fn compile_expr_stmt(ctx: &mut Ctx<'_>, e: &ExprStmt, keep_value: bool) -> Result<(), String> {
    compile_expr(ctx, &e.expr)?;
    if !keep_value {
        // Return 已消耗栈；其余表达式语句丢弃结果。
        if !matches!(e.expr, TermExpression::Return(_)) {
            ctx.f.emit(Op::Pop);
            ctx.f.emit_u8(1);
        }
    }
    Ok(())
}

fn pattern_name(p: &Pattern) -> Result<String, String> {
    match p {
        Pattern::Variable(v) => Ok(v.name.name.clone()),
        _ => Err("暂只支持简单变量绑定模式".into()),
    }
}

fn compile_expr(ctx: &mut Ctx<'_>, expr: &TermExpression) -> Result<(), String> {
    match expr {
        TermExpression::Bool { value, .. } => {
            if *value {
                ctx.f.emit(Op::LoadTrue);
            } else {
                ctx.f.emit(Op::LoadFalse);
            }
        }
        TermExpression::StringLiteral(s) => {
            if let Some(n) = number_from_literal(s) {
                let i = ctx.f.add_const_number(n);
                ctx.f.emit(Op::LoadConst);
                ctx.f.emit_u16(i);
            } else {
                let text = string_literal_text(s)?;
                let i = ctx.f.add_string(text);
                ctx.f.emit(Op::LoadString);
                ctx.f.emit_u16(i);
            }
        }
        TermExpression::NamePath(path) => {
            let name = path_name(path)?;
            if name == "null" {
                ctx.f.emit(Op::LoadNull);
            } else if let Some(&slot) = ctx.locals.get(&name) {
                ctx.f.emit(Op::LoadLocal);
                ctx.f.emit_u16(slot);
            } else if let Some(&fidx) = ctx.fn_index.get(&name) {
                let i = ctx.f.add_const_func(fidx as u32);
                ctx.f.emit(Op::LoadConst);
                ctx.f.emit_u16(i);
            } else {
                let g = ctx.f.add_const_name(name);
                ctx.f.emit(Op::LoadGlobal);
                ctx.f.emit_u16(g);
            }
        }
        TermExpression::Paren { expr, .. } => compile_expr(ctx, expr)?,
        TermExpression::Unary(u) => {
            compile_expr(ctx, &u.base)?;
            match u.operator {
                ValkyrieTokenType::Minus => ctx.f.emit(Op::Neg),
                ValkyrieTokenType::Bang => ctx.f.emit(Op::Not),
                other => return Err(format!("不支持的一元算符：{other:?}")),
            }
        }
        TermExpression::Binary(b) => match b.operator {
            ValkyrieTokenType::AndAnd => compile_and(ctx, &b.lhs, &b.rhs)?,
            ValkyrieTokenType::OrOr => compile_or(ctx, &b.lhs, &b.rhs)?,
            other => {
                compile_expr(ctx, &b.lhs)?;
                compile_expr(ctx, &b.rhs)?;
                let op = match other {
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
                    _ => return Err(format!("不支持的二元算符：{other:?}")),
                };
                ctx.f.emit(op);
            }
        },
        TermExpression::ApplyCall { callee, args, .. } => {
            if let Some(name) = simple_name(callee) {
                if name == "print" || name == "println" {
                    if args.len() != 1 {
                        return Err("print 仅支持单参数".into());
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
            }
            // 一般 callee 表达式
            compile_expr(ctx, callee)?;
            for a in args {
                compile_expr(ctx, a)?;
            }
            ctx.f.emit(Op::Call);
            ctx.f.emit_u8(args.len() as u8);
        }
        TermExpression::Return(r) => {
            if let Some(base) = &r.base {
                compile_expr(ctx, base)?;
            } else {
                ctx.f.emit(Op::LoadNull);
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
                return Err("暂不支持 if-let".into());
            }
            compile_expr(ctx, condition)?;
            ctx.f.emit(Op::JumpIfFalse);
            let jf = ctx.f.len();
            ctx.f.emit_i16(0);
            compile_block_value(ctx, then_branch)?;
            ctx.f.emit(Op::Jump);
            let jend = ctx.f.len();
            ctx.f.emit_i16(0);
            let else_start = ctx.f.len();
            let rel = (else_start as isize) - ((jf + 2) as isize);
            ctx.f.patch_i16(jf, rel as i16);
            if let Some(eb) = else_branch {
                compile_block_value(ctx, eb)?;
            } else {
                ctx.f.emit(Op::LoadNull);
            }
            let end = ctx.f.len();
            let rel2 = (end as isize) - ((jend + 2) as isize);
            ctx.f.patch_i16(jend, rel2 as i16);
        }
        TermExpression::Loop {
            condition,
            pattern,
            body,
            ..
        } => {
            if pattern.is_some() {
                return Err("暂不支持 for/in 循环".into());
            }
            let Some(cond) = condition else {
                return Err("暂不支持无条件 loop".into());
            };
            let loop_start = ctx.f.len();
            compile_expr(ctx, cond)?;
            ctx.f.emit(Op::JumpIfFalse);
            let jf = ctx.f.len();
            ctx.f.emit_i16(0);
            for stmt in &body.statements {
                match stmt {
                    Statement::Let(l) => compile_let(ctx, l)?,
                    Statement::ExprStmt(e) => compile_expr_stmt(ctx, e, false)?,
                }
            }
            ctx.f.emit(Op::Jump);
            let back = ctx.f.len();
            ctx.f.emit_i16(0);
            let end = ctx.f.len();
            let rel_exit = (end as isize) - ((jf + 2) as isize);
            ctx.f.patch_i16(jf, rel_exit as i16);
            let rel_back = (loop_start as isize) - ((back + 2) as isize);
            ctx.f.patch_i16(back, rel_back as i16);
            ctx.f.emit(Op::LoadNull);
        }
        TermExpression::Block(b) => compile_block_value(ctx, b)?,
        other => return Err(format!("暂不支持的表达式：{other:?}")),
    }
    Ok(())
}

fn compile_block_value(ctx: &mut Ctx<'_>, block: &Block) -> Result<(), String> {
    let n = block.statements.len();
    if n == 0 {
        ctx.f.emit(Op::LoadNull);
        return Ok(());
    }
    for (i, stmt) in block.statements.iter().enumerate() {
        let is_last = i + 1 == n;
        match stmt {
            Statement::Let(l) => {
                compile_let(ctx, l)?;
                if is_last {
                    ctx.f.emit(Op::LoadNull);
                }
            }
            Statement::ExprStmt(e) => {
                if is_last && !e.semi && !matches!(e.expr, TermExpression::Return(_)) {
                    compile_expr(ctx, &e.expr)?;
                } else {
                    compile_expr_stmt(ctx, e, false)?;
                    if is_last {
                        ctx.f.emit(Op::LoadNull);
                    }
                }
            }
        }
    }
    Ok(())
}

fn compile_and(ctx: &mut Ctx<'_>, lhs: &TermExpression, rhs: &TermExpression) -> Result<(), String> {
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

fn compile_or(ctx: &mut Ctx<'_>, lhs: &TermExpression, rhs: &TermExpression) -> Result<(), String> {
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

fn path_name(path: &oak_valkyrie::ast::NamePath) -> Result<String, String> {
    if path.parts.len() != 1 {
        return Err(format!(
            "暂不支持限定路径：{}",
            path.parts
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>()
                .join("::")
        ));
    }
    Ok(path.parts[0].name.clone())
}

fn simple_name(expr: &TermExpression) -> Option<String> {
    match expr {
        TermExpression::NamePath(p) if p.parts.len() == 1 => Some(p.parts[0].name.clone()),
        _ => None,
    }
}

fn number_from_literal(s: &StringLiteral) -> Option<f64> {
    if s.quote_count != 0 || s.prefix.is_some() || s.segments.len() != 1 {
        return None;
    }
    match &s.segments[0] {
        StringSegment::Text(t) => t.content.parse::<f64>().ok(),
        _ => None,
    }
}

fn string_literal_text(s: &StringLiteral) -> Result<String, String> {
    let mut out = String::new();
    for seg in &s.segments {
        match seg {
            StringSegment::Text(t) => out.push_str(&t.content),
            StringSegment::Interpolation(_) => {
                return Err("暂不支持字符串插值".into());
            }
        }
    }
    Ok(out)
}
