//! 格式化结果：纯展示数据，不得进入权威状态。

use std::sync::Arc;

use crate::{
    diagnostic::DiagnosticFlags,
    locale::{LocaleId, TextDirection},
};

/// 纯文本本地化结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedText {
    pub text: Arc<str>,
    pub resolved_locale: LocaleId,
    pub direction: TextDirection,
    pub generation: u64,
    pub diagnostics: DiagnosticFlags,
}

impl LocalizedText {
    pub fn plain(text: impl Into<Arc<str>>, resolved_locale: LocaleId, direction: TextDirection, generation: u64) -> Self {
        Self { text: text.into(), resolved_locale, direction, generation, diagnostics: DiagnosticFlags::empty() }
    }

    /// 开发期缺键占位；发行路径应优先 fallback，而不是直接调用此构造。
    pub fn missing_placeholder(message_key: &str, resolved_locale: LocaleId, generation: u64) -> Self {
        let text: Arc<str> = Arc::from(format!("⟦missing:{message_key}⟧"));
        Self { text, resolved_locale, direction: TextDirection::Ltr, generation, diagnostics: DiagnosticFlags::MISSING }
    }
}

/// 富文本安全 token（白名单语义，禁止 HTML / 脚本）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalizedToken {
    Text(Arc<str>),
    Emphasis { text: Arc<str> },
    Icon { id: Arc<str> },
    Link { id: Arc<str>, text: Arc<str> },
}

/// 长期富文本结果模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedContent {
    pub tokens: Arc<[LocalizedToken]>,
    pub resolved_locale: LocaleId,
    pub direction: TextDirection,
    pub generation: u64,
    pub diagnostics: DiagnosticFlags,
}

impl LocalizedContent {
    pub fn from_text(text: LocalizedText) -> Self {
        Self {
            tokens: Arc::from([LocalizedToken::Text(text.text)]),
            resolved_locale: text.resolved_locale,
            direction: text.direction,
            generation: text.generation,
            diagnostics: text.diagnostics,
        }
    }
}
