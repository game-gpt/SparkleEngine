//! 不可变 Locale 快照与切换事件。

use std::sync::Arc;

use crate::{
    bundle::LocalizationBundle,
    diagnostic::DiagnosticFlags,
    document::{MessageDefinition, MessageName},
    eval::evaluate_compiled,
    locale::{LocaleId, TextDirection, build_fallback_chain},
    message::{MessageArgs, MessageId, MessageRef, NamespaceId},
    text::LocalizedText,
};

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
    pub fn empty(locale: LocaleId, product_default: &LocaleId, available: &[LocaleId], generation: u64) -> Self {
        Self::from_bundle(locale, product_default, available, generation, LocalizationBundle::new())
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
        Self { locale, fallback_chain, direction, generation, bundle: Arc::new(bundle) }
    }

    /// 测试用：从扁平字符串表构造（内部编译为 bundle）。
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
            let Ok(entry_locale) = LocaleId::parse(&locale_tag)
            else {
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
        let Some(name) = message_name(message)
        else {
            return LocalizedText::missing_placeholder(&message.to_string(), self.locale.clone(), self.generation);
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

        let Some(compiled) = compiled
        else {
            let mut text = LocalizedText::missing_placeholder(name, self.locale.clone(), self.generation);
            text.diagnostics = DiagnosticFlags::MISSING;
            return text;
        };

        let rendered = evaluate_compiled(compiled, args, &mut diagnostics);
        LocalizedText { text: rendered, resolved_locale, direction: self.direction, generation: self.generation, diagnostics }
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
        Self { snapshot: Arc::new(snapshot) }
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
        LocaleChanged { previous, current, generation }
    }

    pub fn format(&self, message: &MessageRef, args: &MessageArgs) -> LocalizedText {
        self.snapshot.format(message, args)
    }
}
