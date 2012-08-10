//! Lua AST → Spark HIR（子集）。
//!
//! 支持：顶层 `return`、`local`/`=`、算术 `+/-/*//`、字面量、局部变量、`if`（无 elseif）。
//! 不支持的构造返回错误令牌，由调用方回退旧字节码路径。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use oak_lua::ast::{
    LuaAssignmentStatement, LuaExpression, LuaIfStatement, LuaLocalStatement, LuaRoot,
    LuaStatement,
};
use spark_script_ir::{HirBinaryOp, HirExpr, HirFunction, HirModule, HirStmt, PackageId, Ty};

/// 尝试将整个根降低为 HIR。
pub(crate) fn lower_root_to_hir(
    root: &LuaRoot,
    native_names: &[&str],
) -> Result<HirModule, String> {
    let _native_set: HashSet<&str> = native_names.iter().copied().collect();
    // 有 function 定义时走旧路径（后续再扩 IR）。
    for stmt in &root.statements {
        if matches!(stmt, LuaStatement::Function(_)) {
            return Err("ir_unsupported_function_def".into());
        }
    }

    let mut locals: HashMap<String, u32> = HashMap::new();
    let mut local_tys: Vec<(Arc<str>, Ty)> = Vec::new();
    let (body, saw_return) =
        lower_block_stmts(&root.statements, &mut locals, &mut local_tys)?;

    let mut body = body;
    if !saw_return {
        body.push(HirStmt::Return {
            value: Some(HirExpr::LiteralNull { span: None }),
            span: None,
        });
    }

    Ok(HirModule {
        package: PackageId::anonymous(),
        name: Arc::from("main"),
        functions: vec![HirFunction {
            name: Arc::from("__main"),
            symbol: None,
            params: Vec::new(),
            return_ty: Ty::Dynamic,
            locals: local_tys,
            body,
            span: None,
        }],
    })
}

fn lower_block_stmts(
    stmts: &[LuaStatement],
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
) -> Result<(Vec<HirStmt>, bool), String> {
    let mut body = Vec::new();
    let mut saw_return = false;
    for stmt in stmts {
        if matches!(stmt, LuaStatement::Function(_)) {
            continue;
        }
        let (hir, is_ret) = lower_statement(stmt, locals, local_tys)?;
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
) -> Result<(HirStmt, bool), String> {
    match stmt {
        LuaStatement::Return(r) => {
            if r.values.len() != 1 {
                return Err("ir_unsupported_multi_return".into());
            }
            let value = lower_expr(&r.values[0], locals)?;
            Ok((
                HirStmt::Return {
                    value: Some(value),
                    span: None,
                },
                true,
            ))
        }
        LuaStatement::Local(l) => Ok((lower_local(l, locals, local_tys)?, false)),
        LuaStatement::Assignment(a) => Ok((lower_assignment(a, locals)?, false)),
        LuaStatement::If(i) => Ok((lower_if(i, locals, local_tys)?, false)),
        LuaStatement::Function(_) => Err("ir_unsupported_function_def".into()),
        other => Err(format!("ir_unsupported_stmt:{other:?}")),
    }
}

fn lower_local(
    l: &LuaLocalStatement,
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
) -> Result<HirStmt, String> {
    if l.names.len() != 1 {
        return Err("ir_unsupported_multi_local".into());
    }
    let name = l.names[0].clone();
    let value = match l.values.first() {
        Some(e) => lower_expr(e, locals)?,
        None => HirExpr::LiteralNull { span: None },
    };
    let index = if let Some(&idx) = locals.get(&name) {
        idx
    } else {
        let idx = local_tys.len() as u32;
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
    let value = lower_expr(&a.values[0], locals)?;
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
) -> Result<HirStmt, String> {
    if !i.else_ifs.is_empty() {
        return Err("ir_unsupported_elseif".into());
    }
    let cond = lower_expr(&i.condition, locals)?;
    let (then_body, _) = lower_block_stmts(&i.then_block, locals, local_tys)?;
    let else_body = match &i.else_block {
        Some(block) => lower_block_stmts(block, locals, local_tys)?.0,
        None => Vec::new(),
    };
    Ok(HirStmt::If {
        cond,
        then_body,
        else_body,
        span: None,
    })
}

fn lower_expr(
    expr: &LuaExpression,
    locals: &HashMap<String, u32>,
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
        LuaExpression::Identifier(name) => {
            let Some(&index) = locals.get(name) else {
                return Err(format!("ir_unknown_name:{name}"));
            };
            Ok(HirExpr::Local {
                index,
                span: None,
            })
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
                other => return Err(format!("ir_unsupported_binop:{other}")),
            };
            Ok(HirExpr::Binary {
                op: hir_op,
                lhs: Box::new(lower_expr(&b.left, locals)?),
                rhs: Box::new(lower_expr(&b.right, locals)?),
                span: None,
            })
        }
        other => Err(format!("ir_unsupported_expr:{other:?}")),
    }
}
