//! Oaks `oak-valkyrie` Builder → Spark HIR → 公共 IR → `spark-vm` 字节码。
//!
//! 解析只走 [`ValkyrieBuilder`]。优先经 `spark-script-ir` 管线；子集不覆盖时回退旧 lowering。

mod compile;
mod lower;
mod native_sig;

use compile::compile_root;
use lower::lower_root_to_hir;

use oak_core::{Builder, SourceText};
use oak_core::errors::OakErrorKind;
use oak_valkyrie::{ValkyrieBuilder, ValkyrieLanguage, ValkyrieRoot};
use spark_diagnostics::{ErrorArg, ErrorArgs, ErrorContext, SourceSpan};
use spark_script_ir::{emit_module, lower_module};
use spark_vm::Module;

pub use native_sig::{NativeParam, NativeRegistry, NativeSignature, TypeRef};

#[derive(Debug)]
pub enum ValkyrieScriptError {
    /// 解析失败。`args.reason` 为机器令牌（非用户 Locale 句子）。
    Parse {
        args: ErrorArgs,
        span: Option<SourceSpan>,
    },
    /// 编译失败。
    Compile {
        args: ErrorArgs,
        span: Option<SourceSpan>,
    },
}

impl ValkyrieScriptError {
    pub fn parse_failed(diag_count: u64, span: Option<SourceSpan>) -> Self {
        let mut args = ErrorArgs::new()
            .with("reason", ErrorArg::String(std::sync::Arc::from("parse_failed")))
            .with("diagnostics", ErrorArg::Unsigned(diag_count));
        if let Some(span) = span {
            args = args.with("span", ErrorArg::Span(span));
        }
        Self::Parse { args, span }
    }

    pub fn compile_reason(reason: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Compile {
            args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())),
            span: None,
        }
    }

    pub fn compile_opaque(detail: impl Into<std::sync::Arc<str>>) -> Self {
        Self::compile_reason(detail)
    }

    pub fn span(&self) -> Option<SourceSpan> {
        match self {
            Self::Parse { span, .. } | Self::Compile { span, .. } => *span,
        }
    }

    pub fn context(&self) -> ErrorContext {
        let mut ctx = ErrorContext::new().target("spark-script-valkyrie");
        if let Some(span) = self.span() {
            ctx = ctx.with_span(span);
        }
        ctx
    }
}

impl std::fmt::Display for ValkyrieScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse { .. } => f.write_str("spark.script.valkyrie.parse"),
            Self::Compile { .. } => f.write_str("spark.script.valkyrie.compile"),
        }
    }
}

impl std::error::Error for ValkyrieScriptError {}

/// 源码 → [`Module`]。
///
/// `natives` 仅提供函数名，**不足以**支撑补全与类型检查。
/// 新代码请优先使用 [`compile_with_registry`]。
/// 优先走 HIR→MIR→字节码；子集不覆盖则回退旧路径。
pub fn compile(source: &str, natives: &[&str]) -> Result<Module, ValkyrieScriptError> {
    let root = parse(source)?;
    match lower_root_to_hir(&root, natives) {
        Ok(hir) => {
            let mir = lower_module(&hir).map_err(ValkyrieScriptError::compile_opaque)?;
            emit_module(&mir).map_err(ValkyrieScriptError::compile_opaque)
        }
        Err(_) => compile_root(&root, natives).map_err(ValkyrieScriptError::compile_opaque),
    }
}

/// 带完整宿主签名的编译入口。
pub fn compile_with_registry(
    source: &str,
    natives: &NativeRegistry,
) -> Result<Module, ValkyrieScriptError> {
    let names = natives.name_list();
    compile(source, &names)
}

/// 解析为 Oaks [`ValkyrieRoot`]。
pub fn parse(source: &str) -> Result<ValkyrieRoot, ValkyrieScriptError> {
    let language = ValkyrieLanguage::default();
    let builder = ValkyrieBuilder::new(&language);
    let text = SourceText::new(source);
    let mut session = oak_core::ParseSession::<ValkyrieLanguage>::default();
    let out = builder.build(&text, &[], &mut session);
    match out.result {
        Ok(root) => Ok(root),
        Err(e) => Err(ValkyrieScriptError::parse_failed(
            out.diagnostics.len() as u64,
            oak_offset_span(e.kind()),
        )),
    }
}

fn oak_offset_span(kind: &OakErrorKind) -> Option<SourceSpan> {
    let offset = match kind {
        OakErrorKind::SyntaxError { offset, .. }
        | OakErrorKind::UnexpectedCharacter { offset, .. }
        | OakErrorKind::UnexpectedToken { offset, .. }
        | OakErrorKind::UnexpectedEof { offset, .. }
        | OakErrorKind::ExpectedToken { offset, .. }
        | OakErrorKind::ExpectedName { offset, .. }
        | OakErrorKind::TrailingCommaNotAllowed { offset, .. } => *offset,
        _ => return None,
    };
    Some(SourceSpan::new(offset, offset.saturating_add(1)))
}

/// 调试：列出根上 `micro` 名。
pub fn list_micros(source: &str) -> Result<Vec<String>, ValkyrieScriptError> {
    let root = parse(source)?;
    Ok(root
        .items
        .iter()
        .filter_map(|i| match i {
            oak_valkyrie::ast::StatementNode::Micro(m) => Some(m.name.name.clone()),
            _ => None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_vm::{StdHost, Vm};

    #[test]
    fn arithmetic_main() {
        let m = compile("return 40 + 2", &[]).unwrap();
        let mut vm = Vm::new(m);
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn micro_call() {
        let m = compile(
            r#"
            micro add(a, b) {
                return a + b
            }
            return add(40, 2)
            "#,
            &[],
        )
        .unwrap();
        let mut vm = Vm::new(m);
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn short_circuit_and_or() {
        use spark_gc::Value;
        // Oaks 当前对 `false && …` / `if true` 解析不稳，用比较表达式覆盖短路。
        let m = compile("return 1 < 0 && 99", &[]).unwrap();
        let mut vm = Vm::new(m);
        let v = vm.run(&mut StdHost).unwrap();
        assert!(matches!(v, Value::Bool(false)));

        let m = compile("return 1 < 2 || 0", &[]).unwrap();
        let mut vm = Vm::new(m);
        let v = vm.run(&mut StdHost).unwrap();
        assert!(matches!(v, Value::Bool(true)));
    }

    #[test]
    fn if_while_via_ir() {
        let m = compile(
            r#"
            if 1 < 2 {
                return 40 + 2
            }
            return 0
            "#,
            &[],
        )
        .unwrap();
        let mut vm = Vm::new(m);
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(42.0));

        let m = compile(
            r#"
            while 1 < 0 {
                return 1
            }
            return 42
            "#,
            &[],
        )
        .unwrap();
        let mut vm = Vm::new(m);
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn native_register_block() {
        use spark_gc::Value;

        let m = compile(
            r#"register_block(1, "astracraft3:dirt", "泥土", "textures/dirt.png", 1, 1, 30, "none")"#,
            &["register_block"],
        )
        .unwrap();
        let mut vm = Vm::new(m);
        let called = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let c2 = called.clone();
        vm.register_native("register_block", move |_ctx, args: Vec<Value>| {
            assert_eq!(args.len(), 8);
            c2.set(c2.get() + 1);
            Ok(Value::Null)
        });
        let _ = vm.run(&mut StdHost).unwrap();
        assert_eq!(called.get(), 1);
    }

    #[test]
    fn parse_error_carries_source_span() {
        let err = parse("@@@").expect_err("bare attributes");
        assert_eq!(err.to_string(), "spark.script.valkyrie.parse");
        match &err {
            ValkyrieScriptError::Parse { args, span } => {
                assert!(matches!(
                    args.get("reason"),
                    Some(ErrorArg::String(s)) if s.as_ref() == "parse_failed"
                ));
                assert!(span.is_some());
            }
            _ => panic!("expected Parse"),
        }
    }
}
