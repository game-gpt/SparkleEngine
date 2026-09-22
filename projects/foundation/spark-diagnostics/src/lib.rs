//! Spark 结构化错误与诊断。
//!
//! 错误只保存事实（码、类型化参数、上下文、因果）。最终用户句子由
//! `spark-localization` 在渲染边界生成。`Display` 仅输出稳定错误码。

#![forbid(missing_docs)]
mod args;
mod code;
mod context;
mod diagnostic;
mod error;
mod severity;

pub use args::{ErrorArg, ErrorArgs};
pub use code::{ErrorCode, ErrorId, NamespaceId, codes};
pub use context::ErrorContext;
pub use diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticNote, MessageKey, SourceSpan};
pub use error::{Error, ErrorCause};
pub use severity::Severity;
