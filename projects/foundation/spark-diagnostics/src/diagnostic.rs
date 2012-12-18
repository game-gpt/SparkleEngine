//! 可展示诊断（尚未本地化的消息键 + 标签）。

use std::sync::Arc;

use crate::{args::ErrorArgs, code::ErrorCode, error::Error, severity::Severity};

/// 本地化消息键（与 `spark-localization::MessageRef` 对齐的轻量表示）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageKey {
    pub namespace: Arc<str>,
    pub message: Arc<str>,
}

impl MessageKey {
    pub fn new(namespace: impl Into<Arc<str>>, message: impl Into<Arc<str>>) -> Self {
        Self { namespace: namespace.into(), message: message.into() }
    }

    /// 由错误码推导用户消息键：`spark.asset.not_found` → `spark` / `error.asset.not_found`。
    pub fn user_message_for(code: &ErrorCode) -> Self {
        Self::new(code.namespace.as_str(), format!("error.{}", code.id.as_str()))
    }

    pub fn debug_message_for(code: &ErrorCode) -> Self {
        Self::new(code.namespace.as_str(), format!("debug.{}", code.id.as_str()))
    }
}

/// 源码字节范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

impl SourceSpan {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// 诊断主消息旁的源码标签。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    pub span: SourceSpan,
    pub message_key: Option<MessageKey>,
}

/// 附加说明 / 建议。
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticNote {
    pub message_key: MessageKey,
    pub args: ErrorArgs,
}

/// 结构化诊断。
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: ErrorCode,
    pub message: MessageKey,
    pub args: ErrorArgs,
    pub labels: Vec<DiagnosticLabel>,
    pub notes: Vec<DiagnosticNote>,
    pub cause: Option<Error>,
}

impl Diagnostic {
    pub fn error(code: ErrorCode) -> Self {
        let message = MessageKey::user_message_for(&code);
        Self { severity: Severity::Error, code, message, args: ErrorArgs::new(), labels: Vec::new(), notes: Vec::new(), cause: None }
    }

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

    pub fn with_args(mut self, args: ErrorArgs) -> Self {
        self.args = args;
        self
    }

    pub fn note(mut self, key: MessageKey) -> Self {
        self.notes.push(DiagnosticNote { message_key: key, args: ErrorArgs::new() });
        self
    }
}
