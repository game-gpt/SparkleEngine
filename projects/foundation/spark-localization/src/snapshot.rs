//! 不可变 Locale 快照与切换事件。

use std::sync::Arc;

use crate::bundle::LocalizationBundle;
use crate::diagnostic::DiagnosticFlags;
use crate::document::{MessageDefinition, MessageName};
use crate::eval::evaluate_compiled;
use crate::locale::{LocaleId, TextDirection, build_fallback_chain};
use crate::message::{MessageArgs, MessageId, MessageRef, NamespaceId};
use crate::text::LocalizedText;

/// Locale 切换后由 `spark-event` 传播的事件载荷。
///
/// 本 crate 只定义数据；总线发送由 `spark-engine` / 宿主完成。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleChanged {
    pub previous: LocaleId,
    pub current: LocaleId,
    pub generation: u64,
}

/// 运行时不可变快照：读热路径只读此对象。
#[derive(Debug, Clone)]
pub struct LocaleSnapshot {
    pub locale: LocaleId,
    pub fallback_chain: Arc<[LocaleId]>,
    pub direction: TextDirection,
    pub generation: u64,
    bundle: Arc<LocalizationBundle>,
}

impl LocaleSnapshot {
    /// 构造空快照（仅协商结果，无消息）。
    pub fn empty(
        locale: LocaleId,
        product_default: &LocaleId,
        available: &[LocaleId],
        generation: u64,
    ) -> Self {
        Self::from_bundle(
            locale,
            product_default,
            available,
            generation,
            LocalizationBundle::new(),
        )
    }

    /// 从已编译语言包构造快照。
    pub fn from_bundle(
        locale: LocaleId,
        product_default: &LocaleId,
        available: &[LocaleId],
        generation: u64,
        bundle: LocalizationBundle,
    ) -> Self {
        let direction = locale.direction();
        let fallback_chain = build_fallback_chain(&locale, product_default, available);
        Self {
            locale,
            fallback_chain,
            direction,
            generation,
            bundle: Arc::new(bundle),
        }
    }

    /// 测试 / 过渡：从扁平字符串表构造（内部编译为 bundle）。
    ///
    /// `entries`：`(namespace, message, locale_tag, pattern)`。
    pub fn from_entries(
        locale: LocaleId,
        product_default: &LocaleId,
        available: &[LocaleId],
        generation: u64,
        entries: impl IntoIterator<Item = (Arc<str>, Arc<str>, Arc<str>, Arc<str>)>,
    ) -> Self {
        let mut bundle = LocalizationBundle::new();
        for (namespace, message, locale_tag, pattern) in entries {
            let Ok(entry_locale) = LocaleId::parse(&locale_tag) else {
                continue;
            };
            bundle.insert(
                NamespaceId::new(namespace),
                MessageName::new(message),
                entry_locale,
                crate::bundle::CompiledMessage::from_definition(&MessageDefinition::Text(pattern)),
            );
        }
        Self::from_bundle(locale, product_default, available, generation, bundle)
    }

    pub fn bundle(&self) -> &LocalizationBundle {
        &self.bundle
    }

    /// 查询并格式化消息。
    pub fn format(&self, message: &MessageRef, args: &MessageArgs) -> LocalizedText {
        let Some(name) = message_name(message) else {
            return LocalizedText::missing_placeholder(
                &message.to_string(),
                self.locale.clone(),
                self.generation,
            );
        };

        let ns = NamespaceId::new(message.namespace.as_str());
        let msg_name = MessageName::new(name);
        let mut diagnostics = DiagnosticFlags::empty();
        let mut resolved_locale = self.locale.clone();
        let mut compiled = None;

        for candidate in self.fallback_chain.iter() {
            if let Some(found) = self.bundle.get(&ns, &msg_name, candidate) {
                compiled = Some(found);
                resolved_locale = candidate.clone();
                if candidate != &self.locale {
                    diagnostics.insert(DiagnosticFlags::FALLBACK_USED);
                }
                break;
            }
        }

        let Some(compiled) = compiled else {
            let mut text =
                LocalizedText::missing_placeholder(name, self.locale.clone(), self.generation);
            text.diagnostics = DiagnosticFlags::MISSING;
            return text;
        };

        let rendered = evaluate_compiled(compiled, args, &mut diagnostics);
        LocalizedText {
            text: rendered,
            resolved_locale,
            direction: self.direction,
            generation: self.generation,
            diagnostics,
        }
    }
}

fn message_name(message: &MessageRef) -> Option<&str> {
    match &message.message {
        MessageId::Name(name) => Some(name.as_ref()),
        MessageId::Compact(_) => None,
    }
}

/// 持有当前快照；切换时整体替换 `Arc`。
#[derive(Debug, Clone)]
pub struct Localizer {
    snapshot: Arc<LocaleSnapshot>,
}

impl Localizer {
    pub fn new(snapshot: LocaleSnapshot) -> Self {
        Self {
            snapshot: Arc::new(snapshot),
        }
    }

    pub fn snapshot(&self) -> Arc<LocaleSnapshot> {
        Arc::clone(&self.snapshot)
    }

    pub fn generation(&self) -> u64 {
        self.snapshot.generation
    }

    pub fn locale(&self) -> &LocaleId {
        &self.snapshot.locale
    }

    /// 原子替换快照，返回 `LocaleChanged`。
    ///
    /// 调用方应在帧边界执行，并经 `spark-event` 广播。
    pub fn commit(&mut self, next: LocaleSnapshot) -> LocaleChanged {
        let previous = self.snapshot.locale.clone();
        let current = next.locale.clone();
        let generation = next.generation;
        self.snapshot = Arc::new(next);
        LocaleChanged {
            previous,
            current,
            generation,
        }
    }

    pub fn format(&self, message: &MessageRef, args: &MessageArgs) -> LocalizedText {
        self.snapshot.format(message, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{MessageDefinition, MessageNode, SelectKind};
    use crate::locale::{LocaleId, LocaleRequest, negotiate};
    use crate::message::{MessageArgs, MessageRef, MessageValue};
    use crate::compile::compile_documents;
    use crate::compile::CompileOptions;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn loc(tag: &str) -> LocaleId {
        LocaleId::parse(tag).unwrap()
    }

    #[test]
    fn commit_is_atomic_and_bumps_generation() {
        let available = [loc("en"), loc("zh-Hans-CN")];
        let en = LocaleSnapshot::empty(loc("en"), &loc("en"), &available, 1);
        let mut localizer = Localizer::new(en);
        let zh = LocaleSnapshot::from_entries(
            loc("zh-Hans-CN"),
            &loc("en"),
            &available,
            2,
            [(
                Arc::from("astracraft"),
                Arc::from("menu.continue"),
                Arc::from("zh-Hans-CN"),
                Arc::from("继续游戏"),
            )],
        );
        let changed = localizer.commit(zh);
        assert_eq!(changed.previous.as_str(), "en");
        assert_eq!(changed.current.as_str(), "zh-Hans-CN");
        assert_eq!(changed.generation, 2);
        let text = localizer.format(
            &MessageRef::named("astracraft", "menu.continue"),
            &MessageArgs::new(),
        );
        assert_eq!(text.text.as_ref(), "继续游戏");
        assert_eq!(text.generation, 2);
    }

    #[test]
    fn format_uses_fallback_chain() {
        let available = [loc("en"), loc("zh-Hans")];
        let resolved = negotiate(
            &LocaleRequest::new(vec![loc("zh-Hans-CN")], vec![]),
            &available,
            &loc("en"),
        );
        assert_eq!(resolved.as_str(), "zh-Hans");
        let snap = LocaleSnapshot::from_entries(
            resolved,
            &loc("en"),
            &available,
            1,
            [(
                Arc::from("spark"),
                Arc::from("widget.ok"),
                Arc::from("en"),
                Arc::from("OK"),
            )],
        );
        let text = snap.format(&MessageRef::named("spark", "widget.ok"), &MessageArgs::new());
        assert_eq!(text.text.as_ref(), "OK");
        assert!(text.diagnostics.contains(DiagnosticFlags::FALLBACK_USED));
    }

    #[test]
    fn format_substitutes_named_args() {
        let available = [loc("en")];
        let snap = LocaleSnapshot::from_entries(
            loc("en"),
            &loc("en"),
            &available,
            1,
            [(
                Arc::from("game"),
                Arc::from("welcome"),
                Arc::from("en"),
                Arc::from("Hello, {player_name}"),
            )],
        );
        let mut args = MessageArgs::new();
        args.insert("player_name", MessageValue::String(Arc::from("Ada")));
        let text = snap.format(&MessageRef::named("game", "welcome"), &args);
        assert_eq!(text.text.as_ref(), "Hello, Ada");
    }

    #[test]
    fn format_cardinal_select_from_bundle() {
        let mut cases = BTreeMap::new();
        cases.insert(
            Arc::from("one"),
            vec![
                MessageNode::Argument {
                    name: Arc::from("count"),
                    format: crate::document::ArgumentFormat::None,
                },
                MessageNode::Text(Arc::from(" item")),
            ],
        );
        cases.insert(
            Arc::from("other"),
            vec![
                MessageNode::Argument {
                    name: Arc::from("count"),
                    format: crate::document::ArgumentFormat::None,
                },
                MessageNode::Text(Arc::from(" items")),
            ],
        );
        let mut doc = crate::document::LocalizationDocument::new(loc("en"), "game");
        doc.insert(
            "item_count",
            MessageDefinition::Select {
                argument: Arc::from("count"),
                kind: SelectKind::Cardinal,
                cases,
            },
        );
        let bundle = compile_documents(&[doc], CompileOptions::default())
            .unwrap()
            .bundle;
        let snap = LocaleSnapshot::from_bundle(loc("en"), &loc("en"), &[loc("en")], 1, bundle);
        let mut args = MessageArgs::new();
        args.insert("count", MessageValue::Integer(2));
        let text = snap.format(&MessageRef::named("game", "item_count"), &args);
        assert_eq!(text.text.as_ref(), "2 items");
    }
}
