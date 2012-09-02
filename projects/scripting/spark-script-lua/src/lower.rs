//! Lua AST → Spark HIR（子集）。
//!
//! 支持：顶层 / 函数内 `return`、`local`/`=`、算术比较、字面量、局部变量、
//! `if` / `elseif`、`while`、`do` 块、顶层 `function`、脚本调用与宿主调用、`print`。
//! 不支持的构造返回错误令牌，由调用方回退旧字节码路径。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use oak_lua::ast::{
    LuaAssignmentStatement, LuaCallExpression, LuaExpression, LuaFunctionStatement,
    LuaIfStatement, LuaLocalStatement, LuaRoot, LuaStatement, LuaWhileStatement,
};
use spark_script_ir::{
    HirBinaryOp, HirExpr, HirFunction, HirModule, HirStmt, PackageId, Ty,
};

/// 尝试将整个根降低为 HIR。
pub(crate) fn lower_root_to_hir(
    root: &LuaRoot,
    native_names: &[&str],
) -> Result<HirModule, String> {
    let native_set: HashSet<&str> = native_names.iter().copied().collect();
    let mut fn_index: HashMap<String, u32> = HashMap::new();
    let mut fn_count = 0u32;
    for stmt in &root.statements {
        if let LuaStatement::Function(f) = stmt {
            let name = function_name(f)?;
            fn_index.insert(name, fn_count);
            fn_count += 1;
        }
    }

    let mut functions: Vec<HirFunction> = Vec::new();
    for stmt in &root.statements {
        if let LuaStatement::Function(f) = stmt {
            functions.push(lower_function(f, &fn_index, &native_set)?);
        }
    }

    let mut locals: HashMap<String, u32> = HashMap::new();
    let mut local_tys: Vec<(Arc<str>, Ty)> = Vec::new();
    let (mut body, saw_return) =
        lower_block_stmts(&root.statements, &mut locals, &mut local_tys, &fn_index, &native_set)?;

    if !saw_return {
        body.push(HirStmt::Return {
            value: Some(HirExpr::LiteralNull { span: None }),
            span: None,
        });
    }

    functions.push(HirFunction {
        name: Arc::from("__main"),
        symbol: None,
        params: Vec::new(),
        return_ty: Ty::Dynamic,
        locals: local_tys,
        body,
        span: None,
    });

    Ok(HirModule {
        package: PackageId::anonymous(),
        name: Arc::from("main"),
        functions,
    })
}

fn function_name(f: &LuaFunctionStatement) -> Result<String, String> {
    if f.receiver.is_some() {
        return Err("ir_unsupported_method_def".into());
    }
    if f.name.len() != 1 {
        return Err(format!(
            "ir_unsupported_qualified_name:{}",
            f.name.join(".")
        ));
    }
    if f.is_vararg {
        return Err("ir_unsupported_varargs".into());
    }
    Ok(f.name[0].clone())
}

fn lower_function(
    f: &LuaFunctionStatement,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirFunction, String> {
    let name = function_name(f)?;
    let mut locals: HashMap<String, u32> = HashMap::new();
    let mut params: Vec<(Arc<str>, Ty)> = Vec::new();
    for (i, p) in f.parameters.iter().enumerate() {
        locals.insert(p.clone(), i as u32);
        params.push((Arc::from(p.as_str()), Ty::Dynamic));
    }
    let mut local_tys: Vec<(Arc<str>, Ty)> = Vec::new();
    let (mut body, saw_return) =
        lower_block_stmts(&f.block, &mut locals, &mut local_tys, fn_index, native_set)?;
    if !saw_return {
        body.push(HirStmt::Return {
            value: Some(HirExpr::LiteralNull { span: None }),
            span: None,
        });
    }
    Ok(HirFunction {
        name: Arc::from(name),
        symbol: None,
        params,
        return_ty: Ty::Dynamic,
        locals: local_tys,
        body,
        span: None,
    })
}

fn lower_block_stmts(
    stmts: &[LuaStatement],
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<(Vec<HirStmt>, bool), String> {
    let mut body = Vec::new();
    let mut saw_return = false;
    for stmt in stmts {
        if matches!(stmt, LuaStatement::Function(_)) {
            continue;
        }
        if let LuaStatement::Do(block) = stmt {
            let (inner, is_ret) =
                lower_block_stmts(block, locals, local_tys, fn_index, native_set)?;
            if is_ret {
                saw_return = true;
            }
            body.extend(inner);
            continue;
        }
        let (hir, is_ret) = lower_statement(stmt, locals, local_tys, fn_index, native_set)?;
        if is_ret {
            saw_return = true;
        }
        body.push(hir);
    }
    Ok((body, saw_return))
}

fn lower_statement(
    stmt: &LuaStatement,
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<(HirStmt, bool), String> {
    match stmt {
        LuaStatement::Return(r) => {
            if r.values.len() > 1 {
                return Err("ir_unsupported_multi_return".into());
            }
            let value = match r.values.first() {
                Some(e) => Some(lower_expr(e, locals, fn_index, native_set)?),
                None => Some(HirExpr::LiteralNull { span: None }),
            };
            Ok((HirStmt::Return { value, span: None }, true))
        }
        LuaStatement::Local(l) => Ok((
            lower_local(l, locals, local_tys, fn_index, native_set)?,
            false,
        )),
        LuaStatement::Assignment(a) => Ok((
            lower_assignment(a, locals, fn_index, native_set)?,
            false,
        )),
        LuaStatement::If(i) => Ok((lower_if(i, locals, local_tys, fn_index, native_set)?, false)),
        LuaStatement::While(w) => Ok((
            lower_while(w, locals, local_tys, fn_index, native_set)?,
            false,
        )),
        LuaStatement::Expression(e) => {
            let expr = lower_expr(e, locals, fn_index, native_set)?;
            Ok((HirStmt::Expr { expr, span: None }, false))
        }
        LuaStatement::Do(_) => Err("ir_do_should_be_flattened".into()),
        LuaStatement::Function(_) => Err("ir_unsupported_nested_function".into()),
        other => Err(format!("ir_unsupported_stmt:{other:?}")),
    }
}

fn lower_local(
    l: &LuaLocalStatement,
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirStmt, String> {
    if l.names.len() != 1 {
        return Err("ir_unsupported_multi_local".into());
    }
    let name = l.names[0].clone();
    let value = match l.values.first() {
        Some(e) => lower_expr(e, locals, fn_index, native_set)?,
        None => HirExpr::LiteralNull { span: None },
    };
    let index = if let Some(&idx) = locals.get(&name) {
        idx
    } else {
        let idx = locals.len() as u32;
        locals.insert(name.clone(), idx);
        local_tys.push((Arc::from(name.as_str()), Ty::Dynamic));
        idx
    };
    Ok(HirStmt::AssignLocal {
        index,
        value,
        span: None,
    })
}

fn lower_assignment(
    a: &LuaAssignmentStatement,
    locals: &mut HashMap<String, u32>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirStmt, String> {
    if a.targets.len() != 1 || a.values.len() != 1 {
        return Err("ir_unsupported_multi_assign".into());
    }
    let LuaExpression::Identifier(name) = &a.targets[0] else {
        return Err("ir_unsupported_assign_target".into());
    };
    let Some(&index) = locals.get(name) else {
        return Err(format!("ir_unknown_local:{name}"));
    };
    let value = lower_expr(&a.values[0], locals, fn_index, native_set)?;
    Ok(HirStmt::AssignLocal {
        index,
        value,
        span: None,
    })
}

fn lower_if(
    i: &LuaIfStatement,
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirStmt, String> {
    let cond = lower_expr(&i.condition, locals, fn_index, native_set)?;
    let (then_body, _) =
        lower_block_stmts(&i.then_block, locals, local_tys, fn_index, native_set)?;
    let mut else_body = match &i.else_block {
        Some(block) => lower_block_stmts(block, locals, local_tys, fn_index, native_set)?.0,
        None => Vec::new(),
    };
    // elseif 折成嵌套 If：if a then .. else if b then .. else ..
    for (cond_e, block) in i.else_ifs.iter().rev() {
        let cond = lower_expr(cond_e, locals, fn_index, native_set)?;
        let (then_body, _) = lower_block_stmts(block, locals, local_tys, fn_index, native_set)?;
        else_body = vec![HirStmt::If {
            cond,
            then_body,
            else_body,
            span: None,
        }];
    }
    Ok(HirStmt::If {
        cond,
        then_body,
        else_body,
        span: None,
    })
}

fn lower_while(
    w: &LuaWhileStatement,
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirStmt, String> {
    let cond = lower_expr(&w.condition, locals, fn_index, native_set)?;
    let (body, _) = lower_block_stmts(&w.block, locals, local_tys, fn_index, native_set)?;
    Ok(HirStmt::While {
        cond,
        body,
        span: None,
    })
}

fn lower_expr(
    expr: &LuaExpression,
    locals: &HashMap<String, u32>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirExpr, String> {
    match expr {
        LuaExpression::Nil => Ok(HirExpr::LiteralNull { span: None }),
        LuaExpression::Boolean(b) => Ok(HirExpr::LiteralBool {
            value: *b,
            span: None,
        }),
        LuaExpression::Number(n) => Ok(HirExpr::LiteralNumber {
            value: *n,
            span: None,
        }),
        LuaExpression::String(s) => Ok(HirExpr::LiteralString {
            value: Arc::from(s.as_str()),
            span: None,
        }),
        LuaExpression::Identifier(name) => {
            if let Some(&index) = locals.get(name) {
                return Ok(HirExpr::Local {
                    index,
                    span: None,
                });
            }
            if let Some(&fidx) = fn_index.get(name) {
                return Ok(HirExpr::FuncRef {
                    func_index: fidx,
                    span: None,
                });
            }
            Err(format!("ir_unknown_name:{name}"))
        }
        LuaExpression::Binary(b) => {
            let hir_op = match b.op.as_str() {
                "+" => HirBinaryOp::Add,
                "-" => HirBinaryOp::Sub,
                "*" => HirBinaryOp::Mul,
                "/" => HirBinaryOp::Div,
                "==" => HirBinaryOp::Eq,
                "~=" | "!=" => HirBinaryOp::Ne,
                "<" => HirBinaryOp::Lt,
                "<=" => HirBinaryOp::Le,
                ">" => HirBinaryOp::Gt,
                ">=" => HirBinaryOp::Ge,
                "and" | "or" => return Err(format!("ir_unsupported_binop:{}", b.op)),
                other => return Err(format!("ir_unsupported_binop:{other}")),
            };
            Ok(HirExpr::Binary {
                op: hir_op,
                lhs: Box::new(lower_expr(&b.left, locals, fn_index, native_set)?),
                rhs: Box::new(lower_expr(&b.right, locals, fn_index, native_set)?),
                span: None,
            })
        }
        LuaExpression::Unary(u) => {
            use spark_script_ir::HirUnaryOp;
            let op = match u.op.as_str() {
                "-" => HirUnaryOp::Neg,
                "not" => HirUnaryOp::Not,
                other => return Err(format!("ir_unsupported_unary:{other}")),
            };
            Ok(HirExpr::Unary {
                op,
                expr: Box::new(lower_expr(&u.operand, locals, fn_index, native_set)?),
                span: None,
            })
        }
        LuaExpression::Call(c) => lower_call(c, locals, fn_index, native_set),
        other => Err(format!("ir_unsupported_expr:{other:?}")),
    }
}

fn lower_call(
    c: &LuaCallExpression,
    locals: &HashMap<String, u32>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirExpr, String> {
    let LuaExpression::Identifier(name) = &c.function else {
        return Err("ir_unsupported_callee".into());
    };
    let argv: Result<Vec<_>, _> = c
        .arguments
        .iter()
        .map(|a| lower_expr(a, locals, fn_index, native_set))
        .collect();
    let argv = argv?;

    if name == "print" || name == "println" {
        if argv.len() != 1 {
            return Err("ir_print_arity_one".into());
        }
        return Ok(HirExpr::Print {
            value: Box::new(argv.into_iter().next().unwrap()),
            span: None,
        });
    }
    if native_set.contains(name.as_str()) {
        return Ok(HirExpr::HostCall {
            host_name: Arc::from(name.as_str()),
            args: argv,
            span: None,
        });
    }
    if let Some(&fidx) = fn_index.get(name) {
        return Ok(HirExpr::Call {
            callee: Box::new(HirExpr::FuncRef {
                func_index: fidx,
                span: None,
            }),
            args: argv,
            span: None,
        });
    }
    Err(format!("ir_unknown_function:{name}"))
}
