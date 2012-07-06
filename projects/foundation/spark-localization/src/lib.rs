//! Spark 国际化与本地化运行时。
//!
//! 提供 Locale 协商、消息标识、作者文档中间模型、不可变 [`LocaleSnapshot`]
//! 与诊断。不依赖 `spark-widget` / `spark-engine` / 任何游戏仓。
//!
//! # 边界
//!
//! - **负责**：BCP 47、消息求值契约、回退链、切换事务数据、伪本地化钩子位。
//! - **不负责**：机器翻译、玩法权威状态、已格式化文本的网络/存档权威。

mod asset;
mod bundle;
mod check;
mod compile;
mod coverage;
mod diagnostic;
mod document;
mod eval;
mod json;
mod locale;
mod manifest;
mod manifest_von;
mod message;
mod pseudo;
mod render;
mod snapshot;
mod text;

pub use asset::{
    LocaleLoadError, MemoryLocaleLoader, load_bundle_from_manifest, load_document_json,
    prepare_snapshot,
};
pub use bundle::{CompiledMessage, LocalizationBundle};
pub use check::{CheckIssue, CheckIssueKind, CheckReport, check_document, check_locale_set};
pub use compile::{CompileError, CompileOptions, CompileOutput, compile_document, compile_documents};
pub use coverage::{
    CoverageEntry, CoverageReport, CoverageStatus, coverage_against, coverage_set, fallback_key,
};
pub use diagnostic::{DiagnosticFlags, DiagnosticRecord, MessageDiagnostic};
pub use document::{
    ArgumentFormat, LocalizationDocument, MessageDefinition, MessageName, MessageNode, SelectKind,
};
pub use json::{JsonError, document_from_json_slice, document_from_json_str};
pub use locale::{
    LocaleId, LocaleParseError, LocaleRequest, TextDirection, build_fallback_chain, negotiate,
};
pub use manifest::{LocaleEntry, LocalizationManifest, NamespaceOwner};
pub use manifest_von::{ManifestVonError, manifest_from_von_str};
pub use message::{
    AttributeId, MessageArgs, MessageDateTime, MessageDecimal, MessageDuration, MessageId,
    MessageRef, MessageValue, NamespaceId,
};
pub use pseudo::{PseudoKind, generate_pseudo};
pub use render::{message_args_from_error_args, render_diagnostic, render_error};
pub use snapshot::{LocaleChanged, LocaleSnapshot, Localizer};
pub use text::{LocalizedContent, LocalizedText, LocalizedToken};
