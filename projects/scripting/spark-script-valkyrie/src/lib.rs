//! Oaks `oak-valkyrie` Builder → Spark HIR → 公共 IR → `spark-vm` 字节码。
//!
//! 解析只走 [`ValkyrieBuilder`]。正式编译只经 `spark-ir`；不支持的构造必须报错。

#![forbid(missing_docs)]
mod lower;
mod native_sig;

use lower::lower_root_to_hir;

use oak_core::{Builder, SourceText, errors::OakErrorKind};
use oak_valkyrie::{ValkyrieBuilder, ValkyrieLanguage, ValkyrieRoot};
use spark_diagnostics::{ErrorArg, ErrorArgs, ErrorContext, SourceSpan};
use spark_ir::{HostBindTable, HostEmitMode, emit_module_with_host, lower_module};
use spark_vm::Module;

pub use native_sig::{NativeParam, TypeRef};

/// Valkyrie 脚本前端失败：解析或编译任一阶段。
///
/// `Display` 输出稳定错误码（`spark.script.valkyrie.parse` / `.compile`），
/// 人类可读细节在 `args` 的 `reason` 等机器令牌中，不经 Locale。
#[derive(Debug)]
pub enum ValkyrieScriptError {
    /// 解析失败。`args.reason` 为机器令牌（非用户 Locale 句子）。
    Parse {
        /// 诊断参数表（含 `reason`、`diagnostics` 计数，可选 `span`）。
        args: ErrorArgs,
        /// 首个可定位语法错误的源码跨度；未知时为 `None`。
        span: Option<SourceSpan>,
    },
    /// 编译失败（降低 / IR / 发射）。
    Compile {
        /// 诊断参数表；至少含 `reason` 令牌字符串。
        args: ErrorArgs,
        /// 若有源码定位则附带；当前多数路径为 `None`。
        span: Option<SourceSpan>,
    },
}

impl ValkyrieScriptError {
    /// 构造解析失败：`diagnostics` 为 Oaks 诊断条数；`span` 来自首个可映射的 [`OakErrorKind`]。
    pub fn parse_failed(diag_count: u64, span: Option<SourceSpan>) -> Self {
        let mut args = ErrorArgs::new()
            .with("reason", ErrorArg::String(std::sync::Arc::from("parse_failed")))
            .with("diagnostics", ErrorArg::Unsigned(diag_count));
        if let Some(span) = span {
            args = args.with("span", ErrorArg::Span(span));
        }
        Self::Parse { args, span }
    }

    /// 构造编译失败，`reason` 写入 `args`（不透明令牌或降低错误串）。
    pub fn compile_reason(reason: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Compile { args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())), span: None }
    }

    /// [`compile_reason`] 的别名，用于把不透明细节直接当成 `reason`。
    pub fn compile_opaque(detail: impl Into<std::sync::Arc<str>>) -> Self {
        Self::compile_reason(detail)
    }

    /// 错误关联的源码跨度（解析或编译变体上的 `span` 字段）。
    pub fn span(&self) -> Option<SourceSpan> {
        match self {
            Self::Parse { span, .. } | Self::Compile { span, .. } => *span,
        }
    }

    /// 诊断上下文：目标固定为 `spark-script-valkyrie`，有 span 则附带。
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

/// 无宿主绑定的源码 → [`Module`]。有宿主时请用 [`compile_with_binds`]。
pub fn compile(source: &str) -> Result<Module, ValkyrieScriptError> {
    compile_with_binds(source, &HostBindTable::new())
}

/// 带完整宿主绑定表的编译入口（稳定身份 + 槽位）。
pub fn compile_with_binds(source: &str, hosts: &HostBindTable) -> Result<Module, ValkyrieScriptError> {
    let root = parse(source)?;
    let hir = lower_root_to_hir(&root, hosts).map_err(ValkyrieScriptError::compile_opaque)?;
    let mir = lower_module(&hir).map_err(ValkyrieScriptError::compile_opaque)?;
    let mode = if hosts.is_empty() { HostEmitMode::NoHost } else { HostEmitMode::Bound(hosts) };
    emit_module_with_host(&mir, mode).map_err(ValkyrieScriptError::compile_opaque)
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
        Err(e) => Err(ValkyrieScriptError::parse_failed(out.diagnostics.len() as u64, oak_offset_span(e.kind()))),
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
