//! Valkyrie AST → Spark HIR（子集竖切）。
//!
//! 当前支持：顶层 `return` / `let`、算术比较、一元、字面量、局部变量。
//! 含 `micro` / 调用 / `if` / `loop` 等时返回 `Err`，由调用方回退旧路径。

use std::collections::HashMap;
use std::sync::Arc;

use oak_valkyrie::ValkyrieTokenType;
use oak_valkyrie::ast::{
    ExprStmt, Let, Pattern, StatementNode, StringLiteral, StringSegment, TermExpression,
    ValkyrieRoot,
};
use spark_script_ir::{
    HirBinaryOp, HirExpr, HirFunction, HirModule, HirStmt, HirUnaryOp, PackageId, Ty,
};

/// 尝试将整个根降低为 HIR。不支持的构造返回错误令牌。
pub(crate) fn lower_root_to_hir(root: &ValkyrieRoot) -> Result<HirModule, String> {
    for item in &root.items {
        if matches!(item, StatementNode::Micro(_)) {
            return Err("ir_unsupported_micro".into());
        }
    }

    let mut locals: HashMap<String, u32> = HashMap::new();
    let mut local_tys: Vec<(Arc<str>, Ty)> = Vec::new();
    let mut body: Vec<HirStmt> = Vec::new();
    let mut saw_return = false;

    for item in &root.items {
        match item {
            StatementNode::Micro(_) => unreachable!(),
            StatementNode::Let(l) => {
                body.push(lower_let(l, &mut locals, &mut local_tys)?);
            }
            StatementNode::ExprStmt(s) => {
                if matches!(s.expr, TermExpression::Return(_)) {
                    saw_return = true;
                }
                body.push(lower_expr_stmt(s, &locals)?);
            }
            StatementNode::Statement(inner) => match inner.as_ref() {
                StatementNode::Let(l) => {
                    body.push(lower_let(l, &mut locals, &mut local_tys)?);
                }
                StatementNode::ExprStmt(s) => {
                    if matches!(s.expr, TermExpression::Return(_)) {
                        saw_return = true;
                    }
                    body.push(lower_expr_stmt(s, &locals)?);
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

fn lower_let(
    l: &Let,
    locals: &mut HashMap<String, u32>,
    local_tys: &mut Vec<(Arc<str>, Ty)>,
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
        value: lower_expr(&l.expr, locals)?,
        span: None,
    })
}

fn lower_expr_stmt(s: &ExprStmt, locals: &HashMap<String, u32>) -> Result<HirStmt, String> {
    if let TermExpression::Return(r) = &s.expr {
        let value = match &r.base {
            Some(v) => Some(lower_expr(v, locals)?),
            None => Some(HirExpr::LiteralNull { span: None }),
        };
        return Ok(HirStmt::Return { value, span: None });
    }
    Ok(HirStmt::Expr {
        expr: lower_expr(&s.expr, locals)?,
        span: None,
    })
}

fn lower_expr(e: &TermExpression, locals: &HashMap<String, u32>) -> Result<HirExpr, String> {
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
            let index = *locals
                .get(name)
                .ok_or_else(|| format!("ir_unknown_local:{name}"))?;
            Ok(HirExpr::Local { index, span: None })
        }
        TermExpression::Unary(u) => {
            let expr = Box::new(lower_expr(&u.base, locals)?);
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
        TermExpression::Binary(b) => {
            // 短路在前端展开；当前竖切不支持 && / ||。
            if matches!(
                b.operator,
                ValkyrieTokenType::AndAnd | ValkyrieTokenType::OrOr
            ) {
                return Err("ir_unsupported_short_circuit".into());
            }
            let op = match b.operator {
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
                other => return Err(format!("ir_unsupported_binary:{other:?}")),
            };
            Ok(HirExpr::Binary {
                op,
                lhs: Box::new(lower_expr(&b.lhs, locals)?),
                rhs: Box::new(lower_expr(&b.rhs, locals)?),
                span: None,
            })
        }
        TermExpression::Paren { expr, .. } => lower_expr(expr, locals),
        TermExpression::Return(_) => Err("ir_nested_return".into()),
        other => Err(format!("ir_unsupported_expr:{other:?}")),
    }
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
