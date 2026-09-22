//! Oaks `oak-ruby` Builder → 公共 IR → `spark-vm` 字节码。
//!
//! 解析只走 [`RubyBuilder`]。正式编译只经 `spark-ir`；不支持的构造必须报错。

#![warn(missing_docs)]
mod lower;

use lower::lower_root_to_hir;

use oak_core::{Builder, SourceText};
use oak_ruby::{RubyBuilder, RubyLanguage, RubyRoot};
use spark_diagnostics::{ErrorArg, ErrorArgs};
use spark_ir::{HostEmitMode, emit_module_with_host, lower_module};
use spark_vm::Module;

pub use oak_ruby::RubyRoot as ParsedRoot;
pub use spark_ir::HostBindTable;

#[derive(Debug)]
pub enum RubyScriptError {
    Parse { args: ErrorArgs },
    Compile { args: ErrorArgs },
}

impl RubyScriptError {
    pub fn parse_failed(diag_count: u64) -> Self {
        Self::Parse {
            args: ErrorArgs::new()
                .with("reason", ErrorArg::String(std::sync::Arc::from("parse_failed")))
                .with("diagnostics", ErrorArg::Unsigned(diag_count)),
        }
    }

    pub fn compile_reason(reason: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Compile { args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())) }
    }

    pub fn parse_opaque(detail: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Parse { args: ErrorArgs::new().with("reason", ErrorArg::String(detail.into())) }
    }

    pub fn compile_opaque(detail: impl Into<std::sync::Arc<str>>) -> Self {
        Self::compile_reason(detail)
    }
}

impl std::fmt::Display for RubyScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse { args } => {
                write!(f, "spark.script.ruby.parse")?;
                if let Some(ErrorArg::Unsigned(n)) = args.get("diagnostics") {
                    write!(f, ":diagnostics={n}")?;
                }
                if let Some(ErrorArg::String(s)) = args.get("reason") {
                    write!(f, ":{s}")?;
                }
                Ok(())
            }
            Self::Compile { args } => {
                write!(f, "spark.script.ruby.compile")?;
                if let Some(ErrorArg::String(s)) = args.get("reason") {
                    write!(f, ":{s}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for RubyScriptError {}

/// 无宿主绑定的源码 → [`Module`]。有宿主时请用 [`compile_with_binds`]。
pub fn compile(source: &str) -> Result<Module, RubyScriptError> {
    compile_with_binds(source, &HostBindTable::new())
}

/// 带完整宿主绑定表的编译入口。
pub fn compile_with_binds(source: &str, hosts: &HostBindTable) -> Result<Module, RubyScriptError> {
    let root = parse(source)?;
    let hir = lower_root_to_hir(&root, hosts).map_err(RubyScriptError::compile_opaque)?;
    let mir = lower_module(&hir).map_err(RubyScriptError::compile_opaque)?;
    let mode = if hosts.is_empty() { HostEmitMode::NoHost } else { HostEmitMode::Bound(hosts) };
    emit_module_with_host(&mir, mode).map_err(RubyScriptError::compile_opaque)
}

/// 解析为 Oaks AST。
pub fn parse(source: &str) -> Result<RubyRoot, RubyScriptError> {
    let language = RubyLanguage::default();
    let builder = RubyBuilder::new(&language);
    let text = SourceText::new(source);
    let mut session = oak_core::ParseSession::<RubyLanguage>::default();
    let out = builder.build(&text, &[], &mut session);
    match out.result {
        Ok(root) => Ok(root),
        Err(_err) => Err(RubyScriptError::parse_failed(out.diagnostics.len() as u64)),
    }
}
