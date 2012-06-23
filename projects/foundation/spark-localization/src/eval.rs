//! 编译消息求值（读热路径，无 IO）。

use std::fmt::Write;
use std::sync::Arc;

use crate::bundle::CompiledMessage;
use crate::diagnostic::{DiagnosticFlags, MessageDiagnostic};
use crate::document::{MessageNode, SelectKind};
use crate::message::{MessageArgs, MessageValue};

pub(crate) fn evaluate_compiled(
    message: &CompiledMessage,
    args: &MessageArgs,
    diagnostics: &mut DiagnosticFlags,
) -> Arc<str> {
    match message {
        CompiledMessage::Text(text) => text.clone(),
        CompiledMessage::Pattern(nodes) => Arc::from(eval_nodes(nodes, args, diagnostics, &mut Vec::new())),
        CompiledMessage::Select {
            argument,
            kind,
            cases,
        } => {
            let case_key = resolve_select_case(argument, *kind, args, diagnostics);
            let nodes = cases
                .get(case_key.as_ref())
                .or_else(|| cases.get("other"))
                .map(|n| n.as_ref())
                .unwrap_or(&[]);
            Arc::from(eval_nodes(nodes, args, diagnostics, &mut Vec::new()))
        }
    }
}

fn resolve_select_case(
    argument: &str,
    kind: SelectKind,
    args: &MessageArgs,
    diagnostics: &mut DiagnosticFlags,
) -> Arc<str> {
    match args.get(argument) {
        Some(MessageValue::Integer(v)) => match kind {
            SelectKind::Cardinal | SelectKind::Ordinal => {
                if *v == 1 {
                    Arc::from("one")
                } else {
                    Arc::from("other")
                }
            }
            SelectKind::Select => {
                let mut buf = String::new();
                let _ = write!(&mut buf, "{v}");
                Arc::from(buf)
            }
        },
        Some(MessageValue::Select(s)) => s.clone(),
        Some(MessageValue::String(s)) => s.clone(),
        Some(_) => {
            diagnostics.insert(DiagnosticFlags::from_diagnostic(MessageDiagnostic::BadArgument));
            Arc::from("other")
        }
        None => {
            diagnostics.insert(DiagnosticFlags::from_diagnostic(
                MessageDiagnostic::MissingArgument,
            ));
            Arc::from("other")
        }
    }
}

fn eval_nodes(
    nodes: &[MessageNode],
    args: &MessageArgs,
    diagnostics: &mut DiagnosticFlags,
    stack: &mut Vec<Arc<str>>,
) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            MessageNode::Text(text) => out.push_str(text),
            MessageNode::Argument { name, .. } => match args.get(name) {
                Some(MessageValue::String(s)) => out.push_str(s),
                Some(MessageValue::Integer(v)) => {
                    let _ = write!(&mut out, "{v}");
                }
                Some(MessageValue::Select(s)) => out.push_str(s),
                Some(MessageValue::Decimal(d)) => {
                    // 定点简易展示：scale 位小数。
                    let scale = d.scale as usize;
                    if scale == 0 {
                        let _ = write!(&mut out, "{}", d.coefficient);
                    } else {
                        let div = 10i128.pow(d.scale);
                        let whole = d.coefficient / div;
                        let frac = (d.coefficient % div).abs();
                        let _ = write!(&mut out, "{whole}.{frac:0scale$}");
                    }
                }
                Some(_) => {
                    diagnostics.insert(DiagnosticFlags::from_diagnostic(MessageDiagnostic::BadArgument));
                    out.push('?');
                }
                None => {
                    diagnostics.insert(DiagnosticFlags::from_diagnostic(
                        MessageDiagnostic::MissingArgument,
                    ));
                    out.push('?');
                }
            },
            MessageNode::MessageRef { name, .. } => {
                // 同包消息引用在快照层解析；节点层仅标记。
                if stack.iter().any(|s| s.as_ref() == name.as_str()) {
                    diagnostics.insert(DiagnosticFlags::from_diagnostic(MessageDiagnostic::Cycle));
                    out.push('?');
                } else {
                    diagnostics.insert(DiagnosticFlags::from_diagnostic(MessageDiagnostic::Missing));
                    out.push('?');
                }
            }
            MessageNode::Select {
                argument,
                kind,
                cases,
            } => {
                let case_key = resolve_select_case(argument, *kind, args, diagnostics);
                let child = cases
                    .get(case_key.as_ref())
                    .or_else(|| cases.get("other"))
                    .map(|n| n.as_slice())
                    .unwrap_or(&[]);
                out.push_str(&eval_nodes(child, args, diagnostics, stack));
            }
        }
    }
    out
}
