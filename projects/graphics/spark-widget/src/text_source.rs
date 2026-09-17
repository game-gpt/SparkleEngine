//! 声明式文本数据源：字面量 vs 本地化消息。
//!
//! Widget 不解析语言包；格式化由调用方持有的 [`LocaleSnapshot`] 完成。

use std::sync::Arc;

use spark_localization::{
    AttributeId, LocaleSnapshot, LocalizedText, MessageArgs, MessageRef, TextDirection,
};

/// 文本节点的数据来源。
#[derive(Debug, Clone, PartialEq)]
pub enum TextSource {
    /// 不翻译：调试、玩家输入、文件名、UGC 等。
    Literal(Arc<str>),
    /// 绑定本地化消息。
    Message {
        id: MessageRef,
        args: MessageArgs,
        attribute: Option<AttributeId>,
    },
}

impl TextSource {
    pub fn literal(text: impl Into<Arc<str>>) -> Self {
        Self::Literal(text.into())
    }

    pub fn message(id: MessageRef, args: MessageArgs) -> Self {
        Self::Message {
            id,
            args,
            attribute: None,
        }
    }

    /// 是否在 Locale generation 变化时应失效。
    pub fn depends_on_locale(&self) -> bool {
        matches!(self, Self::Message { .. })
    }

    /// 相对某代快照解析展示文本。
    ///
    /// `attribute` 首版尚未求值属性通道，仍走主消息体。
    pub fn resolve(&self, snapshot: &LocaleSnapshot) -> ResolvedText {
        match self {
            Self::Literal(text) => ResolvedText {
                text: text.clone(),
                direction: TextDirection::Ltr,
                generation: snapshot.generation,
                from_message: false,
            },
            Self::Message { id, args, .. } => {
                let localized: LocalizedText = snapshot.format(id, args);
                ResolvedText {
                    text: localized.text,
                    direction: localized.direction,
                    generation: localized.generation,
                    from_message: true,
                }
            }
        }
    }
}

/// Widget 测量 / 绘制使用的已解析文本。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedText {
    pub text: Arc<str>,
    pub direction: TextDirection,
    pub generation: u64,
    pub from_message: bool,
}

/// 缓存的文本绑定：Locale generation 变化时使 Message 绑定失效。
#[derive(Debug, Clone)]
pub struct TextBinding {
    pub source: TextSource,
    cached: Option<ResolvedText>,
    cached_generation: u64,
}

impl TextBinding {
    pub fn new(source: TextSource) -> Self {
        Self {
            source,
            cached: None,
            cached_generation: 0,
        }
    }

    pub fn invalidate(&mut self) {
        self.cached = None;
    }

    /// 若 generation 变化且源依赖 Locale，则重新解析。
    pub fn resolve(&mut self, snapshot: &LocaleSnapshot) -> &ResolvedText {
        let stale = self.cached.is_none()
            || (self.source.depends_on_locale() && self.cached_generation != snapshot.generation);
        if stale {
            let resolved = self.source.resolve(snapshot);
            self.cached_generation = snapshot.generation;
            self.cached = Some(resolved);
        }
        self.cached.as_ref().expect("cache filled")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_localization::{LocaleId, LocaleSnapshot, MessageArgs, MessageRef, MessageValue};
    use std::sync::Arc;

    fn snap(generation: u64) -> LocaleSnapshot {
        let available = [LocaleId::parse("en").unwrap()];
        LocaleSnapshot::from_entries(
            LocaleId::parse("en").unwrap(),
            &LocaleId::parse("en").unwrap(),
            &available,
            generation,
            [(
                Arc::from("ui"),
                Arc::from("ok"),
                Arc::from("en"),
                Arc::from("OK"),
            )],
        )
    }

    #[test]
    fn literal_ignores_locale_generation() {
        let mut binding = TextBinding::new(TextSource::literal("debug"));
        let a = binding.resolve(&snap(1)).clone();
        let b = binding.resolve(&snap(2)).clone();
        assert_eq!(a.text.as_ref(), "debug");
        assert_eq!(b.text.as_ref(), "debug");
        assert!(!a.from_message);
    }

    #[test]
    fn message_resolves_via_snapshot() {
        let source = TextSource::message(MessageRef::named("ui", "ok"), MessageArgs::new());
        let resolved = source.resolve(&snap(3));
        assert_eq!(resolved.text.as_ref(), "OK");
        assert!(resolved.from_message);
        assert_eq!(resolved.generation, 3);
    }

    #[test]
    fn message_args_flow() {
        let available = [LocaleId::parse("en").unwrap()];
        let snapshot = LocaleSnapshot::from_entries(
            LocaleId::parse("en").unwrap(),
            &LocaleId::parse("en").unwrap(),
            &available,
            1,
            [(
                Arc::from("ui"),
                Arc::from("hi"),
                Arc::from("en"),
                Arc::from("Hi {name}"),
            )],
        );
        let mut args = MessageArgs::new();
        args.insert("name", MessageValue::String(Arc::from("Ada")));
        let text = TextSource::message(MessageRef::named("ui", "hi"), args).resolve(&snapshot);
        assert_eq!(text.text.as_ref(), "Hi Ada");
    }
}
