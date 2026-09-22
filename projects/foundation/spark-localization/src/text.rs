//! 格式化结果：纯展示数据，不得进入权威状态。

use std::sync::Arc;

use crate::{
    diagnostic::DiagnosticFlags,
    locale::{LocaleId, TextDirection},
};

/// 纯文本本地化结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedText {
    /// 已求值展示串（非玩法权威）。
    pub text: Arc<str>,
    /// 实际命中的 Locale（可能是回退链上的节点）。
    pub resolved_locale: LocaleId,
    /// 建议书写方向。
    pub direction: TextDirection,
    /// 产出时快照代数。
    pub generation: u64,
    /// 求值过程累积的诊断位。
    pub diagnostics: DiagnosticFlags,
}

impl LocalizedText {
    /// 无诊断的纯文本结果。
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
    /// 普通文本。
    Text(Arc<str>),
    /// 强调片段。
    Emphasis {
        /// 强调范围内的文本。
        text: Arc<str>,
    },
    /// 图标引用（逻辑资源 id）。
    Icon {
        /// 图标逻辑 id。
        id: Arc<str>,
    },
    /// 可点击链接（逻辑 id + 可见文本）。
    Link {
        /// 链接逻辑 id（非 URL 权威）。
        id: Arc<str>,
        /// 可见锚文本。
        text: Arc<str>,
    },
}

/// 长期富文本结果模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedContent {
    /// 白名单 token 序列。
    pub tokens: Arc<[LocalizedToken]>,
    /// 实际命中的 Locale。
    pub resolved_locale: LocaleId,
    /// 建议书写方向。
    pub direction: TextDirection,
    /// 产出时快照代数。
    pub generation: u64,
    /// 求值诊断位。
    pub diagnostics: DiagnosticFlags,
}

impl LocalizedContent {
    /// 将纯文本结果包成单 [`LocalizedToken::Text`] 的富文本。
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
