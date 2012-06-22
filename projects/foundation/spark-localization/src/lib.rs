//! Spark 国际化与本地化运行时。
//!
//! 提供 Locale 协商、消息标识、作者文档中间模型、不可变 [`LocaleSnapshot`]
//! 与诊断。不依赖 `spark-widget` / `spark-engine` / 任何游戏仓。
//!
//! # 边界
//!
//! - **负责**：BCP 47、消息求值契约、回退链、切换事务数据、伪本地化钩子位。
//! - **不负责**：机器翻译、玩法权威状态、已格式化文本的网络/存档权威。

mod check;
mod diagnostic;
mod document;
mod locale;
mod manifest;
mod message;
mod pseudo;
mod snapshot;
mod text;

pub use check::{CheckIssue, CheckIssueKind, CheckReport, check_document, check_locale_set};
pub use diagnostic::{DiagnosticFlags, DiagnosticRecord, MessageDiagnostic};
pub use document::{
    ArgumentFormat, LocalizationDocument, MessageDefinition, MessageName, MessageNode, SelectKind,
};
pub use locale::{
    LocaleId, LocaleParseError, LocaleRequest, TextDirection, build_fallback_chain, negotiate,
};
pub use manifest::{LocaleEntry, LocalizationManifest, NamespaceOwner};
pub use message::{
    AttributeId, MessageArgs, MessageDateTime, MessageDecimal, MessageDuration, MessageId,
    MessageRef, MessageValue, NamespaceId,
};
pub use pseudo::{PseudoKind, generate_pseudo};
pub use snapshot::{LocaleChanged, LocaleSnapshot, Localizer};
pub use text::{LocalizedContent, LocalizedText, LocalizedToken};
