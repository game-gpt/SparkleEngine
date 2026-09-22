//! 可展示诊断（尚未本地化的消息键 + 标签）。

use std::sync::Arc;

use crate::{args::ErrorArgs, code::ErrorCode, error::Error, severity::Severity};

/// 本地化消息键（与 `spark-localization::MessageRef` 对齐的轻量表示）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageKey {
    /// 消息目录命名空间（通常与错误码命名空间一致）。
    pub namespace: Arc<str>,
    /// 目录内消息标识（如 `error.asset.not_found`）。
    pub message: Arc<str>,
}

impl MessageKey {
    /// 构造消息键；两段均应为稳定英文标识。
    pub fn new(namespace: impl Into<Arc<str>>, message: impl Into<Arc<str>>) -> Self {
        Self { namespace: namespace.into(), message: message.into() }
    }

    /// 由错误码推导用户消息键：`spark.asset.not_found` → `spark` / `error.asset.not_found`。
    pub fn user_message_for(code: &ErrorCode) -> Self {
        Self::new(code.namespace.as_str(), format!("error.{}", code.id.as_str()))
    }

    /// 由错误码推导调试消息键：`spark` / `debug.<id>`（开发者向细节）。
    pub fn debug_message_for(code: &ErrorCode) -> Self {
        Self::new(code.namespace.as_str(), format!("debug.{}", code.id.as_str()))
    }
}

/// 源码字节范围（半开区间 `[start, end)`，单位：UTF-8 字节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    /// 起始字节偏移（含）。
    pub start: usize,
    /// 结束字节偏移（不含）。
    pub end: usize,
}

impl SourceSpan {
    /// 构造半开区间；调用方保证 `start <= end`。
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// 诊断主消息旁的源码标签。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    /// 高亮的源码范围。
    pub span: SourceSpan,
    /// 可选的次级消息键（无则只画下划线）。
    pub message_key: Option<MessageKey>,
}

/// 附加说明 / 建议。
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticNote {
    /// 说明文案的本地化键。
    pub message_key: MessageKey,
    /// 说明插值参数。
    pub args: ErrorArgs,
}

/// 结构化诊断（可渲染，尚未本地化）。
///
/// 由 [`Error`] 提升而来或直接构造；渲染器消费本类型生成用户可见输出。
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    /// 严重级别。
    pub severity: Severity,
    /// 关联的稳定错误码。
    pub code: ErrorCode,
    /// 主消息本地化键。
    pub message: MessageKey,
    /// 主消息插值参数。
    pub args: ErrorArgs,
    /// 源码高亮标签。
    pub labels: Vec<DiagnosticLabel>,
    /// 附加说明列表。
    pub notes: Vec<DiagnosticNote>,
    /// 可选的原始结构化错误（便于再提取因果）。
    pub cause: Option<Error>,
}

impl Diagnostic {
    /// 以 [`Severity::Error`] 与用户消息键构造空诊断。
    pub fn error(code: ErrorCode) -> Self {
        let message = MessageKey::user_message_for(&code);
        Self { severity: Severity::Error, code, message, args: ErrorArgs::new(), labels: Vec::new(), notes: Vec::new(), cause: None }
    }

    /// 从结构化错误提升：复制码与参数，severity为 Error，并保留原错误为 `cause`。
    pub fn from_error(error: Error) -> Self {
        let message = MessageKey::user_message_for(&error.code);
        Self {
            severity: Severity::Error,
            code: error.code.clone(),
            message,
            args: error.args.clone(),
            labels: Vec::new(),
            notes: Vec::new(),
            cause: Some(error),
        }
    }

    /// 替换主消息参数表。
    pub fn with_args(mut self, args: ErrorArgs) -> Self {
        self.args = args;
        self
    }

    /// 追加无额外参数的说明 note。
    pub fn note(mut self, key: MessageKey) -> Self {
        self.notes.push(DiagnosticNote { message_key: key, args: ErrorArgs::new() });
        self
    }
}
