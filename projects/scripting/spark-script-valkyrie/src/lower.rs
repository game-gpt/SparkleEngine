//! Valkyrie AST → Spark HIR。
//!
//! 支持：`micro`、`let`/`return`、算术比较、一元、字面量、局部变量、
//! 脚本调用、宿主调用、`print`、`if`/`while`、以及展开后的 `&&`/`||`。
//! 不支持的构造返回错误令牌，由调用方回退旧路径。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use oak_valkyrie::ValkyrieTokenType;
use oak_valkyrie::ast::{
    Block, ExprStmt, Let, MicroDeclaration, Pattern, Statement, StatementNode, StringLiteral,
    StringSegment, TermExpression, ValkyrieRoot,
};
use spark_script_ir::{
    HirBinaryOp, HirExpr, HirFunction, HirModule, HirStmt, HirUnaryOp, PackageId, Ty,
};

/// 尝试将整个根降低为 HIR。
pub(crate) fn lower_root_to_hir(
    root: &ValkyrieRoot,
    native_names: &[&str],
) -> Result<HirModule, String> {
    let native_set: HashSet<&str> = native_names.iter().copied().collect();
    let mut fn_index: HashMap<String, u32> = HashMap::new();
    let mut micro_count = 0u32;
    for item in &root.items {
        if let StatementNode::Micro(m) = item {
            fn_index.insert(m.name.name.clone(), micro_count);
            micro_count += 1;
        }
    }

    let mut functions: Vec<HirFunction> = Vec::new();
    for item in &root.items {
        if let StatementNode::Micro(m) = item {
            functions.push(lower_micro(m, &fn_index, &native_set)?);
        }
    }

    let mut locals: HashMap<String, u32> = HashMap::new();
    let mut local_tys: Vec<(Arc<str>, Ty)> = Vec::new();
    let mut body: Vec<HirStmt> = Vec::new();
    let mut saw_return = false;

    for item in &root.items {
        match item {
            StatementNode::Micro(_) => {}
            StatementNode::Let(l) => {
                body.push(lower_let(
                    l,
                    &mut locals,
                    &mut local_tys,
                    &fn_index,
                    &native_set,
                )?);
            }
            StatementNode::ExprStmt(s) => {
                let (stmt, is_ret) =
                    lower_expr_stmt(s, &mut locals, &mut local_tys, &fn_index, &native_set)?;
                if is_ret {
                    saw_return = true;
                }
                body.push(stmt);
            }
            StatementNode::Statement(inner) => match inner.as_ref() {
                StatementNode::Let(l) => {
                    body.push(lower_let(
                        l,
                        &mut locals,
                        &mut local_tys,
                        &fn_index,
                        &native_set,
                    )?);
                }
                StatementNode::ExprStmt(s) => {
                    let (stmt, is_ret) =
                        lower_expr_stmt(s, &mut locals, &mut local_tys, &fn_index, &native_set)?;
                    if is_ret {
                        saw_return = true;
                    }
                    body.push(stmt);
                }
                other => return Err(format!("ir_unsupported_root:{other:?}")),
            },
            other => return Err(format!("ir_unsupported_root:{other:?}")),
        }
    }

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

fn lower_micro(
    m: &MicroDeclaration,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirFunction, String> {
    let mut locals: HashMap<String, u32> = HashMap::new();
    let mut params: Vec<(Arc<str>, Ty)> = Vec::new();
    for (i, p) in m.params.iter().enumerate() {
        locals.insert(p.name.name.clone(), i as u32);
        params.push((Arc::from(p.name.name.as_str()), Ty::Dynamic));
    }
    let mut local_tys: Vec<(Arc<str>, Ty)> = Vec::new();
    let (body, _) = lower_block_stmts(
        &m.body,
        &mut locals,
        &mut local_tys,
        fn_index,
        native_set,
        true,
    )?;
    Ok(HirFunction {
        name: Arc::from(m.name.name.as_str()),
        symbol: None,
        params,
        return_ty: Ty::Dynamic,
        locals: local_tys,
        body,
        span: None,
    })
}

fn lower_block_stmts(
    body: &Block,
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
    ensure_return: bool,
) -> Result<(Vec<HirStmt>, bool), String> {
    let mut out = Vec::new();
    let mut saw_return = false;
    for s in &body.statements {
        match s {
            Statement::Let(l) => {
                out.push(lower_let(l, locals, local_tys, fn_index, native_set)?);
            }
            Statement::ExprStmt(e) => {
                let (stmt, is_ret) =
                    lower_expr_stmt(e, locals, local_tys, fn_index, native_set)?;
                if is_ret {
                    saw_return = true;
                }
                out.push(stmt);
            }
        }
    }
    if ensure_return && !saw_return {
        out.push(HirStmt::Return {
            value: Some(HirExpr::LiteralNull { span: None }),
            span: None,
        });
        saw_return = true;
    }
    Ok((out, saw_return))
}

fn lower_let(
    l: &Let,
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirStmt, String> {
    let name = match &l.pattern {
        Pattern::Variable(v) => v.name.name.clone(),
        other => return Err(format!("ir_unsupported_let_pattern:{other:?}")),
    };
    let index = if let Some(&i) = locals.get(&name) {
        i
    } else {
        let i = locals.len() as u32;
        locals.insert(name.clone(), i);
        local_tys.push((Arc::from(name.as_str()), Ty::Dynamic));
        i
    };
    Ok(HirStmt::AssignLocal {
        index,
        value: lower_expr(&l.expr, locals, fn_index, native_set)?,
        span: None,
    })
}

fn lower_expr_stmt(
    s: &ExprStmt,
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<(HirStmt, bool), String> {
    match &s.expr {
        TermExpression::Return(r) => {
            let value = match &r.base {
                Some(v) => Some(lower_expr(v, locals, fn_index, native_set)?),
                None => Some(HirExpr::LiteralNull { span: None }),
            };
            Ok((HirStmt::Return { value, span: None }, true))
        }
        TermExpression::If {
            pattern,
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            if pattern.is_some() {
                return Err("ir_unsupported_if_let".into());
            }
            let cond = lower_expr(condition, locals, fn_index, native_set)?;
            let (then_body, _) =
                lower_block_stmts(then_branch, locals, local_tys, fn_index, native_set, false)?;
            let else_body = if let Some(eb) = else_branch {
                lower_block_stmts(eb, locals, local_tys, fn_index, native_set, false)?.0
            } else {
                Vec::new()
            };
            Ok((
                HirStmt::If {
                    cond,
                    then_body,
                    else_body,
                    span: None,
                },
                false,
            ))
        }
        TermExpression::Loop {
            condition,
            pattern,
            body,
            ..
        } => {
            if pattern.is_some() {
                return Err("ir_unsupported_for_loop".into());
            }
            let Some(cond_expr) = condition else {
                return Err("ir_unsupported_infinite_loop".into());
            };
            let cond = lower_expr(cond_expr, locals, fn_index, native_set)?;
            let (loop_body, _) =
                lower_block_stmts(body, locals, local_tys, fn_index, native_set, false)?;
            Ok((
                HirStmt::While {
                    cond,
                    body: loop_body,
                    span: None,
                },
                false,
            ))
        }
        _ => Ok((
            HirStmt::Expr {
                expr: lower_expr(&s.expr, locals, fn_index, native_set)?,
                span: None,
            },
            false,
        )),
    }
}

fn lower_expr(
    e: &TermExpression,
    locals: &HashMap<String, u32>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirExpr, String> {
    match e {
        TermExpression::Bool { value: true, .. } => Ok(HirExpr::LiteralBool {
            value: true,
            span: None,
        }),
        TermExpression::Bool { value: false, .. } => Ok(HirExpr::LiteralBool {
            value: false,
            span: None,
        }),
        TermExpression::StringLiteral(lit) => lower_literal(lit),
        TermExpression::NamePath(path) => {
            if path.parts.len() != 1 {
                return Err("ir_unsupported_qualified_name".into());
            }
            let name = &path.parts[0].name;
            if name == "null" {
                return Ok(HirExpr::LiteralNull { span: None });
            }
            if let Some(&idx) = locals.get(name) {
                return Ok(HirExpr::Local {
                    index: idx,
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
        TermExpression::Unary(u) => {
            let expr = Box::new(lower_expr(&u.base, locals, fn_index, native_set)?);
            let op = match u.operator {
                ValkyrieTokenType::Minus => HirUnaryOp::Neg,
                ValkyrieTokenType::Bang => HirUnaryOp::Not,
                other => return Err(format!("ir_unsupported_unary:{other:?}")),
            };
            Ok(HirExpr::Unary {
                op,
                expr,
                span: None,
            })
        }
        TermExpression::Binary(b) => match b.operator {
            ValkyrieTokenType::AndAnd => Ok(HirExpr::If {
                cond: Box::new(lower_expr(&b.lhs, locals, fn_index, native_set)?),
                then_branch: Box::new(lower_expr(&b.rhs, locals, fn_index, native_set)?),
                else_branch: Box::new(HirExpr::LiteralBool {
                    value: false,
                    span: None,
                }),
                span: None,
            }),
            ValkyrieTokenType::OrOr => Ok(HirExpr::If {
                cond: Box::new(lower_expr(&b.lhs, locals, fn_index, native_set)?),
                then_branch: Box::new(HirExpr::LiteralBool {
                    value: true,
                    span: None,
                }),
                else_branch: Box::new(lower_expr(&b.rhs, locals, fn_index, native_set)?),
                span: None,
            }),
            other => {
                let op = match other {
                    ValkyrieTokenType::Plus => HirBinaryOp::Add,
                    ValkyrieTokenType::Minus => HirBinaryOp::Sub,
                    ValkyrieTokenType::Star => HirBinaryOp::Mul,
                    ValkyrieTokenType::Slash => HirBinaryOp::Div,
                    ValkyrieTokenType::EqEq => HirBinaryOp::Eq,
                    ValkyrieTokenType::NotEq => HirBinaryOp::Ne,
                    ValkyrieTokenType::LessThan => HirBinaryOp::Lt,
                    ValkyrieTokenType::LessEq => HirBinaryOp::Le,
                    ValkyrieTokenType::GreaterThan => HirBinaryOp::Gt,
                    ValkyrieTokenType::GreaterEq => HirBinaryOp::Ge,
                    _ => return Err(format!("ir_unsupported_binary:{other:?}")),
                };
                Ok(HirExpr::Binary {
                    op,
                    lhs: Box::new(lower_expr(&b.lhs, locals, fn_index, native_set)?),
                    rhs: Box::new(lower_expr(&b.rhs, locals, fn_index, native_set)?),
                    span: None,
                })
            }
        },
        TermExpression::Paren { expr, .. } => lower_expr(expr, locals, fn_index, native_set),
        TermExpression::ApplyCall { callee, args, .. } => {
            lower_call(callee, args, locals, fn_index, native_set)
        }
        TermExpression::Block(body) => {
            // 表达式块：隔离局部（与旧前端「块内 let 外溢」不同，避免复杂别名）。
            let mut block_locals = locals.clone();
            let mut block_tys = Vec::new();
            let (stmts, _) = lower_block_stmts(
                body,
                &mut block_locals,
                &mut block_tys,
                fn_index,
                native_set,
                false,
            )?;
            Ok(HirExpr::Block {
                stmts,
                result: Some(Box::new(HirExpr::LiteralNull { span: None })),
                span: None,
            })
        }
        TermExpression::Return(_) => Err("ir_nested_return".into()),
        other => Err(format!("ir_unsupported_expr:{other:?}")),
    }
}

fn lower_call(
    callee: &TermExpression,
    args: &[TermExpression],
    locals: &HashMap<String, u32>,
    fn_index: &HashMap<String, u32>,
    native_set: &HashSet<&str>,
) -> Result<HirExpr, String> {
    let name = match callee {
        TermExpression::NamePath(path) if path.parts.len() == 1 => path.parts[0].name.clone(),
        other => return Err(format!("ir_unsupported_callee:{other:?}")),
    };
    let argv: Result<Vec<_>, _> = args
        .iter()
        .map(|a| lower_expr(a, locals, fn_index, native_set))
        .collect();
    let argv = argv?;

    if name == "print" || name == "puts" {
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
            host_name: Arc::from(name),
            args: argv,
            span: None,
        });
    }
    if let Some(&fidx) = fn_index.get(&name) {
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

fn lower_literal(lit: &StringLiteral) -> Result<HirExpr, String> {
    let text = plain_text(lit)?;
    if lit.quote_count == 0 {
        if text == "null" {
            return Ok(HirExpr::LiteralNull { span: None });
        }
        let n: f64 = text
            .parse()
            .map_err(|_| format!("ir_invalid_number:{text}"))?;
        return Ok(HirExpr::LiteralNumber {
            value: n,
            span: None,
        });
    }
    Ok(HirExpr::LiteralString {
        value: Arc::from(text),
        span: None,
    })
}

fn plain_text(lit: &StringLiteral) -> Result<String, String> {
    let mut out = String::new();
    for seg in &lit.segments {
        match seg {
            StringSegment::Text(t) => out.push_str(&t.content),
            StringSegment::Interpolation(_) => {
                return Err("ir_unsupported_string_interpolation".into());
            }
        }
    }
    Ok(out)
}
