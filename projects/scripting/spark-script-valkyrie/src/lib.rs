//! Valkyrie 风格子集 → `spark-vm` 字节码。
//!
//! 上游 `oak-valkyrie` Builder 对表达式语句（含 EOF）还原仍不稳定，故本前端用手写
//! 递归下降覆盖 `micro` / `let` / 调用 / 控制流，与 `spark-script-ruby` 同策略。
//! 游戏绑定经原生函数表注入。

mod ast;
mod compile;
mod native_sig;
mod parse;

use compile::compile_root;
use parse::parse as parse_source;

use spark_diagnostics::{ErrorArg, ErrorArgs, ErrorContext, SourceSpan};
use spark_vm::Module;

pub use ast::ValkyrieRoot;
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
    pub fn parse_reason(reason: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Parse {
            args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())),
            span: None,
        }
    }

    pub fn parse_at(reason: impl Into<std::sync::Arc<str>>, span: SourceSpan) -> Self {
        Self::Parse {
            args: ErrorArgs::new()
                .with("reason", ErrorArg::String(reason.into()))
                .with("span", ErrorArg::Span(span)),
            span: Some(span),
        }
    }

    pub fn compile_reason(reason: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Compile {
            args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())),
            span: None,
        }
    }

    pub fn parse_opaque(detail: impl Into<std::sync::Arc<str>>) -> Self {
        Self::parse_reason(detail)
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
pub fn compile(source: &str, natives: &[&str]) -> Result<Module, ValkyrieScriptError> {
    let root = parse(source)?;
    compile_root(&root, natives).map_err(ValkyrieScriptError::compile_opaque)
}

/// 带完整宿主签名的编译入口。
///
/// 当前字节码生成仍只消费函数名；签名供后续名称解析 / 类型检查 / 编辑器复用。
pub fn compile_with_registry(
    source: &str,
    natives: &NativeRegistry,
) -> Result<Module, ValkyrieScriptError> {
    let names = natives.name_list();
    compile(source, &names)
}

/// 解析为 AST 根。
pub fn parse(source: &str) -> Result<ValkyrieRoot, ValkyrieScriptError> {
    parse_source(source).map_err(|fail| ValkyrieScriptError::parse_at(fail.reason, fail.span))
}

/// 调试：列出根上 `micro` 名。
pub fn list_micros(source: &str) -> Result<Vec<String>, ValkyrieScriptError> {
    let root = parse(source)?;
    Ok(root
        .items
        .iter()
        .filter_map(|i| match i {
            ast::Item::Micro(m) => Some(m.name.clone()),
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
        let err = parse("return @").expect_err("illegal char");
        let span = err.span().expect("span");
        assert_eq!(span.start, 7);
        assert_eq!(span.end, 8);
        assert_eq!(err.to_string(), "spark.script.valkyrie.parse");
        match &err {
            ValkyrieScriptError::Parse { args, .. } => {
                assert!(matches!(
                    args.get("reason"),
                    Some(ErrorArg::String(s)) if s.as_ref().starts_with("illegal_char:")
                ));
                assert!(matches!(args.get("span"), Some(ErrorArg::Span(_))));
            }
            _ => panic!("expected Parse"),
        }
    }
}
