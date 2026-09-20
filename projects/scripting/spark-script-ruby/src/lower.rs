//! Ruby AST → Spark HIR（子集）。
//!
//! 支持：顶层 / `def` 内 `return`、局部赋值、算术比较、字面量、
//! `if` / `while` / `until` / `break`、数值 `for .. in a..b`、`&&`/`||`/`and`/`or`、无接收者方法调用与宿主调用、`puts`/`print`/`p`。
//! 类、实例变量、全局、`Send`、块、`each`/`case` 等回退旧路径。
//!
//! 注：Oaks 当前 builder 不产出 `Case`，且 `case`/`when` 解析会卡死，故不走 IR。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use oak_ruby::ast::{ExpressionNode, LiteralNode, RubyRoot, StatementNode};
use spark_script_ir::{
    HirBinaryOp, HirExpr, HirFunction, HirModule, HirStmt, HirUnaryOp, PackageId, Ty,
};

/// 尝试将整个根降低为 HIR。
pub(crate) fn lower_root_to_hir(
    root: &RubyRoot,
    native_names: &[&str],
) -> Result<HirModule, String> {
    let native_set: HashSet<&str> = native_names.iter().copied().collect();
    let mut methods = Vec::new();
    collect_top_methods(&root.statements, &mut methods)?;

    let mut fn_index: HashMap<String, u32> = HashMap::new();
    for (i, m) in methods.iter().enumerate() {
        fn_index.insert(m.name.clone(), i as u32);
    }

    let mut functions: Vec<HirFunction> = Vec::new();
    for method in &methods {
        functions.push(lower_method(method, &fn_index, &native_set)?);
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

struct MethodRef {
    name: String,
    params: Vec<String>,
    body: Vec<StatementNode>,
}

fn collect_top_methods(stmts: &[StatementNode], out: &mut Vec<MethodRef>) -> Result<(), String> {
    for stmt in stmts {
        match stmt {
            StatementNode::MethodDef {
                name, params, body, ..
            } => {
                // 嵌套 def / 类内方法走旧路径。
                if body
                    .iter()
                    .any(|s| matches!(s, StatementNode::MethodDef { .. } | StatementNode::ClassDef { .. }))
                {
                    return Err("ir_unsupported_nested_def".into());
                }
                out.push(MethodRef {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                });
            }
            StatementNode::ClassDef { .. } => {
                return Err("ir_unsupported_class".into());
            }
            StatementNode::If {
                then_body,
                else_body,
                ..
            } => {
                collect_top_methods(then_body, out)?;
                if let Some(else_body) = else_body {
                    collect_top_methods(else_body, out)?;
                }
            }
            StatementNode::While { body, .. } => {
                collect_top_methods(body, out)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn lower_method(
    method: &MethodRef,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirFunction, String> {
    let mut locals: HashMap<String, u32> = HashMap::new();
    let mut params: Vec<(Arc<str>, Ty)> = Vec::new();
    for (i, p) in method.params.iter().enumerate() {
        locals.insert(p.clone(), i as u32);
        params.push((Arc::from(p.as_str()), Ty::Dynamic));
    }
    let mut local_tys: Vec<(Arc<str>, Ty)> = Vec::new();
    let (mut body, saw_return) =
        lower_block_stmts(&method.body, &mut locals, &mut local_tys, fn_index, native_set)?;
    if !saw_return {
        body.push(HirStmt::Return {
            value: Some(HirExpr::LiteralNull { span: None }),
            span: None,
        });
    }
    Ok(HirFunction {
        name: Arc::from(method.name.as_str()),
        symbol: None,
        params,
        return_ty: Ty::Dynamic,
        locals: local_tys,
        body,
        span: None,
    })
}

fn lower_block_stmts(
    stmts: &[StatementNode],
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<(Vec<HirStmt>, bool), String> {
    let mut body = Vec::new();
    let mut saw_return = false;
    for stmt in stmts {
        if matches!(
            stmt,
            StatementNode::MethodDef { .. } | StatementNode::ClassDef { .. }
        ) {
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
    stmt: &StatementNode,
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<(HirStmt, bool), String> {
    match stmt {
        StatementNode::Return { value, .. } => {
            let value = match value {
                Some(e) => Some(lower_expr(e, locals, fn_index, native_set)?),
                None => Some(HirExpr::LiteralNull { span: None }),
            };
            Ok((HirStmt::Return { value, span: None }, true))
        }
        StatementNode::Assignment { target, value, .. } => Ok((
            lower_assignment(target, value, locals, local_tys, fn_index, native_set)?,
            false,
        )),
        StatementNode::If {
            condition,
            then_body,
            else_body,
            ..
        } => Ok((
            lower_if(condition, then_body, else_body.as_deref(), locals, local_tys, fn_index, native_set)?,
            false,
        )),
        StatementNode::While {
            condition, body, ..
        } => Ok((
            lower_while(condition, body, false, locals, local_tys, fn_index, native_set)?,
            false,
        )),
        StatementNode::Until {
            condition, body, ..
        } => Ok((
            lower_while(condition, body, true, locals, local_tys, fn_index, native_set)?,
            false,
        )),
        StatementNode::Expression(expr) => {
            let expr = lower_expr(expr, locals, fn_index, native_set)?;
            Ok((HirStmt::Expr { expr, span: None }, false))
        }
        StatementNode::For {
            var,
            iterable,
            body,
            ..
        } => Ok((
            lower_for_range(var, iterable, body, locals, local_tys, fn_index, native_set)?,
            false,
        )),
        StatementNode::Break { .. } => Ok((HirStmt::Break { span: None }, false)),
        StatementNode::Case { .. }
        | StatementNode::Next { .. }
        | StatementNode::Redo { .. }
        | StatementNode::MethodDef { .. }
        | StatementNode::ClassDef { .. } => Err(format!("ir_unsupported_stmt:{stmt:?}")),
    }
}

fn alloc_local(
    name: &str,
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
) -> u32 {
    if let Some(&idx) = locals.get(name) {
        return idx;
    }
    let idx = locals.len() as u32;
    locals.insert(name.to_string(), idx);
    local_tys.push((Arc::from(name), Ty::Dynamic));
    idx
}

/// `for i in a..b` / `a...b` → while + 自增。其它可迭代回退。
fn lower_for_range(
    var: &str,
    iterable: &ExpressionNode,
    body: &[StatementNode],
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirStmt, String> {
    let ExpressionNode::BinaryOp {
        left,
        operator,
        right,
        ..
    } = iterable
    else {
        return Err("ir_unsupported_for_iterable".into());
    };
    let cmp = match operator.as_str() {
        ".." => HirBinaryOp::Le,
        "..." => HirBinaryOp::Lt,
        _ => return Err(format!("ir_unsupported_for_op:{operator}")),
    };
    let i_idx = alloc_local(var, locals, local_tys);
    let end_name = format!("__for_end_{i_idx}");
    let end_idx = alloc_local(&end_name, locals, local_tys);
    let start = lower_expr(left, locals, fn_index, native_set)?;
    let end = lower_expr(right, locals, fn_index, native_set)?;
    let (mut loop_body, _) = lower_block_stmts(body, locals, local_tys, fn_index, native_set)?;
    loop_body.push(HirStmt::AssignLocal {
        index: i_idx,
        value: HirExpr::Binary {
            op: HirBinaryOp::Add,
            lhs: Box::new(HirExpr::Local {
                index: i_idx,
                span: None,
            }),
            rhs: Box::new(HirExpr::LiteralNumber {
                value: 1.0,
                span: None,
            }),
            span: None,
        },
        span: None,
    });
    Ok(HirStmt::Expr {
        expr: HirExpr::Block {
            stmts: vec![
                HirStmt::AssignLocal {
                    index: i_idx,
                    value: start,
                    span: None,
                },
                HirStmt::AssignLocal {
                    index: end_idx,
                    value: end,
                    span: None,
                },
                HirStmt::While {
                    cond: HirExpr::Binary {
                        op: cmp,
                        lhs: Box::new(HirExpr::Local {
                            index: i_idx,
                            span: None,
                        }),
                        rhs: Box::new(HirExpr::Local {
                            index: end_idx,
                            span: None,
                        }),
                        span: None,
                    },
                    body: loop_body,
                    span: None,
                },
            ],
            result: Some(Box::new(HirExpr::LiteralNull { span: None })),
            span: None,
        },
        span: None,
    })
}

fn lower_assignment(
    target: &str,
    value: &ExpressionNode,
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirStmt, String> {
    if target.starts_with('$') || target.starts_with('@') {
        return Err(format!("ir_unsupported_assign_target:{target}"));
    }
    let value = lower_expr(value, locals, fn_index, native_set)?;
    let index = if let Some(&idx) = locals.get(target) {
        idx
    } else {
        let idx = locals.len() as u32;
        locals.insert(target.to_string(), idx);
        local_tys.push((Arc::from(target), Ty::Dynamic));
        idx
    };
    Ok(HirStmt::AssignLocal {
        index,
        value,
        span: None,
    })
}

fn lower_if(
    condition: &ExpressionNode,
    then_body: &[StatementNode],
    else_body: Option<&[StatementNode]>,
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirStmt, String> {
    let cond = lower_expr(condition, locals, fn_index, native_set)?;
    let (then_body, _) = lower_block_stmts(then_body, locals, local_tys, fn_index, native_set)?;
    let else_body = match else_body {
        Some(block) => lower_block_stmts(block, locals, local_tys, fn_index, native_set)?.0,
        None => Vec::new(),
    };
    Ok(HirStmt::If {
        cond,
        then_body,
        else_body,
        span: None,
    })
}

fn lower_while(
    condition: &ExpressionNode,
    body: &[StatementNode],
    invert: bool,
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirStmt, String> {
    let cond = lower_expr(condition, locals, fn_index, native_set)?;
    let cond = if invert {
        HirExpr::Unary {
            op: HirUnaryOp::Not,
            expr: Box::new(cond),
            span: None,
        }
    } else {
        cond
    };
    let (body, _) = lower_block_stmts(body, locals, local_tys, fn_index, native_set)?;
    Ok(HirStmt::While {
        cond,
        body,
        span: None,
    })
}

fn lower_expr(
    expr: &ExpressionNode,
    locals: &HashMap<String, u32>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirExpr, String> {
    match expr {
        ExpressionNode::Literal(lit) => lower_literal(lit),
        ExpressionNode::Identifier { name, .. } => {
            if name.starts_with('$') || name.starts_with('@') {
                return Err(format!("ir_unsupported_name:{name}"));
            }
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
        ExpressionNode::BinaryOp {
            left,
            operator,
            right,
            ..
        } => {
            if matches!(operator.as_str(), "&&" | "and" | "||" | "or") {
                let lhs = lower_expr(left, locals, fn_index, native_set)?;
                let rhs = lower_expr(right, locals, fn_index, native_set)?;
                let is_and = matches!(operator.as_str(), "&&" | "and");
                return Ok(if is_and {
                    HirExpr::If {
                        cond: Box::new(lhs.clone()),
                        then_branch: Box::new(rhs),
                        else_branch: Box::new(lhs),
                        span: None,
                    }
                } else {
                    HirExpr::If {
                        cond: Box::new(lhs.clone()),
                        then_branch: Box::new(lhs),
                        else_branch: Box::new(rhs),
                        span: None,
                    }
                });
            }
            let hir_op = match operator.as_str() {
                "+" => HirBinaryOp::Add,
                "-" => HirBinaryOp::Sub,
                "*" => HirBinaryOp::Mul,
                "/" => HirBinaryOp::Div,
                "%" => HirBinaryOp::Mod,
                "==" => HirBinaryOp::Eq,
                "!=" | "<>" => HirBinaryOp::Ne,
                "<" => HirBinaryOp::Lt,
                "<=" => HirBinaryOp::Le,
                ">" => HirBinaryOp::Gt,
                ">=" => HirBinaryOp::Ge,
                ".." | "..." => {
                    return Err(format!("ir_unsupported_binop:{operator}"))
                }
                other => return Err(format!("ir_unsupported_binop:{other}")),
            };
            Ok(HirExpr::Binary {
                op: hir_op,
                lhs: Box::new(lower_expr(left, locals, fn_index, native_set)?),
                rhs: Box::new(lower_expr(right, locals, fn_index, native_set)?),
                span: None,
            })
        }
        ExpressionNode::UnaryOp {
            operator, operand, ..
        } => {
            let op = match operator.as_str() {
                "-" => HirUnaryOp::Neg,
                "!" | "not" => HirUnaryOp::Not,
                other => return Err(format!("ir_unsupported_unary:{other}")),
            };
            Ok(HirExpr::Unary {
                op,
                expr: Box::new(lower_expr(operand, locals, fn_index, native_set)?),
                span: None,
            })
        }
        ExpressionNode::MethodCall {
            receiver,
            method,
            args,
            block_body,
            ..
        } => {
            if block_body.is_some() {
                return Err("ir_unsupported_block".into());
            }
            if receiver.is_some() {
                return Err("ir_unsupported_receiver_call".into());
            }
            // 无参裸名且已是局部：按变量读（Ruby 遮蔽规则）。
            if args.is_empty() {
                if let Some(&index) = locals.get(method) {
                    return Ok(HirExpr::Local {
                        index,
                        span: None,
                    });
                }
            }
            lower_bare_call(method, args, locals, fn_index, native_set)
        }
        ExpressionNode::Array { .. } | ExpressionNode::Hash { .. } => {
            Err(format!("ir_unsupported_expr:{expr:?}"))
        }
    }
}

fn lower_literal(lit: &LiteralNode) -> Result<HirExpr, String> {
    match lit {
        LiteralNode::Integer { value, .. } => Ok(HirExpr::LiteralNumber {
            value: *value as f64,
            span: None,
        }),
        LiteralNode::Float { value, .. } => Ok(HirExpr::LiteralNumber {
            value: *value,
            span: None,
        }),
        LiteralNode::String { value, .. } | LiteralNode::Symbol { value, .. } => {
            Ok(HirExpr::LiteralString {
                value: Arc::from(value.as_str()),
                span: None,
            })
        }
        LiteralNode::Boolean { value, .. } => Ok(HirExpr::LiteralBool {
            value: *value,
            span: None,
        }),
        LiteralNode::Nil { .. } => Ok(HirExpr::LiteralNull { span: None }),
    }
}

fn lower_bare_call(
    name: &str,
    args: &[ExpressionNode],
    locals: &HashMap<String, u32>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirExpr, String> {
    let argv: Result<Vec<_>, _> = args
        .iter()
        .map(|a| lower_expr(a, locals, fn_index, native_set))
        .collect();
    let argv = argv?;

    if name == "print" || name == "puts" || name == "p" {
        let value = argv.into_iter().next().unwrap_or(HirExpr::LiteralNull { span: None });
        return Ok(HirExpr::Print {
            value: Box::new(value),
            span: None,
        });
    }
    if native_set.contains(name) {
        return Ok(HirExpr::HostCall {
            host_name: Arc::from(name),
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
