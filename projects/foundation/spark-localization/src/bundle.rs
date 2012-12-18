//! 编译后的运行时语言包（逻辑类型，磁盘扩展名由资产管线决定）。

use std::{collections::BTreeMap, sync::Arc};

use crate::{
    document::{MessageDefinition, MessageName, MessageNode, SelectKind},
    locale::LocaleId,
    message::NamespaceId,
};

/// 单条已编译消息（作者糖已展开）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledMessage {
    Text(Arc<str>),
    Pattern(Arc<[MessageNode]>),
    Select { argument: Arc<str>, kind: SelectKind, cases: BTreeMap<Arc<str>, Arc<[MessageNode]>> },
}

impl CompiledMessage {
    pub fn from_definition(def: &MessageDefinition) -> Self {
        match def {
            MessageDefinition::Text(text) => {
                // 模板糖在文档层可先展开；此处再兜底一次。
                match MessageDefinition::from_template_sugar(text) {
                    MessageDefinition::Text(t) => Self::Text(t),
                    MessageDefinition::Pattern(nodes) => Self::Pattern(Arc::from(nodes)),
                    MessageDefinition::Select { argument, kind, cases } => Self::from_select(argument, kind, &cases),
                }
            }
            MessageDefinition::Pattern(nodes) => Self::Pattern(Arc::from(nodes.as_slice())),
            MessageDefinition::Select { argument, kind, cases } => Self::from_select(argument.clone(), *kind, cases),
        }
    }

    fn from_select(argument: Arc<str>, kind: SelectKind, cases: &BTreeMap<Arc<str>, Vec<MessageNode>>) -> Self {
        let mut compiled = BTreeMap::new();
        for (key, nodes) in cases {
            compiled.insert(key.clone(), Arc::from(nodes.as_slice()));
        }
        Self::Select { argument, kind, cases: compiled }
    }
}

/// 运行时消息包集合。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalizationBundle {
    /// 包格式版本。
    pub format_version: u32,
    /// 内容指纹（确定性，用于缓存键）。
    pub content_hash: u64,
    messages: BTreeMap<BundleKey, CompiledMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct BundleKey {
    namespace: NamespaceId,
    message: MessageName,
    locale: LocaleId,
}

impl LocalizationBundle {
    pub const FORMAT_VERSION: u32 = 1;

    pub fn new() -> Self {
        Self { format_version: Self::FORMAT_VERSION, content_hash: 0, messages: BTreeMap::new() }
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn insert(&mut self, namespace: NamespaceId, message: MessageName, locale: LocaleId, compiled: CompiledMessage) {
        self.messages.insert(BundleKey { namespace, message, locale }, compiled);
        self.rehash();
    }

    pub fn get(&self, namespace: &NamespaceId, message: &MessageName, locale: &LocaleId) -> Option<&CompiledMessage> {
        self.messages.get(&BundleKey { namespace: namespace.clone(), message: message.clone(), locale: locale.clone() })
    }

    pub fn get_named(&self, namespace: &str, message: &str, locale: &LocaleId) -> Option<&CompiledMessage> {
        self.get(&NamespaceId::new(namespace), &MessageName::new(message), locale)
    }

    pub fn locales(&self) -> Vec<LocaleId> {
        let mut set = BTreeMap::new();
        for key in self.messages.keys() {
            set.insert(key.locale.clone(), ());
        }
        set.into_keys().collect()
    }

    pub fn contains(&self, namespace: &NamespaceId, message: &MessageName, locale: &LocaleId) -> bool {
        self.get(namespace, message, locale).is_some()
    }

    /// 后写入覆盖同键。用于按依赖序合并模组语言包。
    pub fn merge_from(&mut self, other: &Self) {
        for (key, message) in &other.messages {
            self.messages.insert(key.clone(), message.clone());
        }
        self.rehash();
    }

    fn rehash(&mut self) {
        // FNV-1a 64：确定性、无额外依赖。
        let mut hash: u64 = 0xcbf29ce484222325;
        let mix = |h: &mut u64, bytes: &[u8]| {
            for b in bytes {
                *h ^= u64::from(*b);
                *h = h.wrapping_mul(0x100000001b3);
            }
        };
        mix(&mut hash, &self.format_version.to_le_bytes());
        for (key, msg) in &self.messages {
            mix(&mut hash, key.namespace.as_str().as_bytes());
            mix(&mut hash, key.message.as_str().as_bytes());
            mix(&mut hash, key.locale.as_str().as_bytes());
            mix(&mut hash, format!("{msg:?}").as_bytes());
        }
        self.content_hash = hash;
    }
}
