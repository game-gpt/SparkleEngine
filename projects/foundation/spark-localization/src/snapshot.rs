//! 不可变 Locale 快照与切换事件。

use std::sync::Arc;

use crate::diagnostic::{DiagnosticFlags, MessageDiagnostic};
use crate::locale::{LocaleId, TextDirection, build_fallback_chain};
use crate::message::{MessageArgs, MessageRef};
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
    /// 已装载命名空间 → 消息名 → 原文 pattern 文本（首版占位字典）。
    ///
    /// 后续替换为编译后的消息字节码；公共 API 不暴露实现细节。
    bundles: Arc<BundleIndex>,
}

#[derive(Debug, Default, Clone)]
struct BundleIndex {
    /// namespace → message name → locale → pattern
    by_ns: std::collections::BTreeMap<Arc<str>, std::collections::BTreeMap<Arc<str>, std::collections::BTreeMap<Arc<str>, Arc<str>>>>,
}

impl LocaleSnapshot {
    /// 构造空快照（仅协商结果，无消息）。
    pub fn empty(locale: LocaleId, product_default: &LocaleId, available: &[LocaleId], generation: u64) -> Self {
        let direction = locale.direction();
        let fallback_chain = build_fallback_chain(&locale, product_default, available);
        Self {
            locale,
            fallback_chain,
            direction,
            generation,
            bundles: Arc::new(BundleIndex::default()),
        }
    }

    /// 从已校验的字符串表构造快照。
    ///
    /// `entries`：`(namespace, message, locale_tag, pattern)`。
    pub fn from_entries(
        locale: LocaleId,
        product_default: &LocaleId,
        available: &[LocaleId],
        generation: u64,
        entries: impl IntoIterator<Item = (Arc<str>, Arc<str>, Arc<str>, Arc<str>)>,
    ) -> Self {
        let mut bundles = BundleIndex::default();
        for (namespace, message, locale_tag, pattern) in entries {
            bundles
                .by_ns
                .entry(namespace)
                .or_default()
                .entry(message)
                .or_default()
                .insert(locale_tag, pattern);
        }
        let direction = locale.direction();
        let fallback_chain = build_fallback_chain(&locale, product_default, available);
        Self {
            locale,
            fallback_chain,
            direction,
            generation,
            bundles: Arc::new(bundles),
        }
    }

    /// 查询并做最小占位格式化（仅替换 `{name}` 字符串/整数；完整规则后续接入）。
    pub fn format(&self, message: &MessageRef, args: &MessageArgs) -> LocalizedText {
        let Some(name) = message_name(message) else {
            return LocalizedText::missing_placeholder(
                &message.to_string(),
                self.locale.clone(),
                self.generation,
            );
        };

        let ns = message.namespace.as_str();
        let mut diagnostics = DiagnosticFlags::empty();
        let mut resolved_locale = self.locale.clone();
        let mut pattern: Option<Arc<str>> = None;

        for candidate in self.fallback_chain.iter() {
            if let Some(found) = self.lookup(ns, name, candidate.as_str()) {
                pattern = Some(found);
                resolved_locale = candidate.clone();
                if candidate != &self.locale {
                    diagnostics.insert(DiagnosticFlags::FALLBACK_USED);
                }
                break;
            }
        }

        let Some(pattern) = pattern else {
            let mut text = LocalizedText::missing_placeholder(name, self.locale.clone(), self.generation);
            text.diagnostics = DiagnosticFlags::MISSING;
            return text;
        };

        let rendered = render_simple_pattern(&pattern, args, &mut diagnostics);
        LocalizedText {
            text: Arc::from(rendered),
            resolved_locale,
            direction: self.direction,
            generation: self.generation,
            diagnostics,
        }
    }

    fn lookup(&self, namespace: &str, message: &str, locale_tag: &str) -> Option<Arc<str>> {
        self.bundles
            .by_ns
            .get(namespace)?
            .get(message)?
            .get(locale_tag)
            .cloned()
    }
}

fn message_name(message: &MessageRef) -> Option<&str> {
    match &message.message {
        crate::message::MessageId::Name(name) => Some(name.as_ref()),
        crate::message::MessageId::Compact(_) => None,
    }
}

fn render_simple_pattern(pattern: &str, args: &MessageArgs, diagnostics: &mut DiagnosticFlags) -> String {
    let mut out = String::with_capacity(pattern.len());
    let mut rest = pattern;
    while let Some(start) = rest.find('{') {
        let (head, after) = rest.split_at(start);
        out.push_str(head);
        let Some(end) = after.find('}') else {
            out.push_str(after);
            diagnostics.insert(DiagnosticFlags::from_diagnostic(MessageDiagnostic::InvalidMarkup));
            return out;
        };
        let key = after[1..end].trim();
        match args.get(key) {
            Some(crate::message::MessageValue::String(s)) => out.push_str(s),
            Some(crate::message::MessageValue::Integer(v)) => {
                use std::fmt::Write;
                let _ = write!(&mut out, "{v}");
            }
            Some(crate::message::MessageValue::Select(s)) => out.push_str(s),
            Some(_) => {
                diagnostics.insert(DiagnosticFlags::from_diagnostic(MessageDiagnostic::BadArgument));
                out.push('?');
            }
            None => {
                diagnostics.insert(DiagnosticFlags::from_diagnostic(MessageDiagnostic::MissingArgument));
                out.push('?');
            }
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
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
    use crate::locale::{LocaleId, LocaleRequest, negotiate};
    use crate::message::{MessageArgs, MessageRef, MessageValue};
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
        let text = localizer.format(&MessageRef::named("astracraft", "menu.continue"), &MessageArgs::new());
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
}
