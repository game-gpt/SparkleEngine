//! 编译后的运行时语言包（逻辑类型，磁盘扩展名由资产管线决定）。

use std::{collections::BTreeMap, sync::Arc};

use crate::{
    document::{MessageDefinition, MessageName, MessageNode, SelectKind},
    locale::LocaleId,
    message::NamespaceId,
};

/// 单条已编译消息（作者糖已展开）。
///
/// 求值路径只读此枚举；不再保留模板糖原文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledMessage {
    /// 无插值的纯文本。
    Text(Arc<str>),
    /// 按序求值的节点序列。
    Pattern(Arc<[MessageNode]>),
    /// 按参数分支选择的消息。
    Select {
        /// 驱动分支的参数名。
        argument: Arc<str>,
        /// 分支语义（显式 / 基数 / 序数）。
        kind: SelectKind,
        /// 分支键 → 节点序列；须含 `other` 兜底（检查阶段强制）。
        cases: BTreeMap<Arc<str>, Arc<[MessageNode]>>,
    },
}

impl CompiledMessage {
    /// 由作者侧定义编译；对 [`MessageDefinition::Text`] 再兜底展开模板糖。
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
    /// 当前包格式版本号；写入 [`Self::format_version`]。
    pub const FORMAT_VERSION: u32 = 1;

    /// 构造空包（版本号为 [`Self::FORMAT_VERSION`]，哈希为 0）。
    pub fn new() -> Self {
        Self { format_version: Self::FORMAT_VERSION, content_hash: 0, messages: BTreeMap::new() }
    }

    /// 已编译消息条目数（命名空间 × 消息 × Locale）。
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// 是否无任何消息。
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// 插入或覆盖一条消息，并重算 [`Self::content_hash`]。
    pub fn insert(&mut self, namespace: NamespaceId, message: MessageName, locale: LocaleId, compiled: CompiledMessage) {
        self.messages.insert(BundleKey { namespace, message, locale }, compiled);
        self.rehash();
    }

    /// 按命名空间 / 消息名 / Locale 精确查找。
    pub fn get(&self, namespace: &NamespaceId, message: &MessageName, locale: &LocaleId) -> Option<&CompiledMessage> {
        self.messages.get(&BundleKey { namespace: namespace.clone(), message: message.clone(), locale: locale.clone() })
    }

    /// 字符串入口的精确查找（内部构造 [`NamespaceId`] / [`MessageName`]）。
    pub fn get_named(&self, namespace: &str, message: &str, locale: &LocaleId) -> Option<&CompiledMessage> {
        self.get(&NamespaceId::new(namespace), &MessageName::new(message), locale)
    }

    /// 包内出现过的 Locale 去重列表（按标签序）。
    pub fn locales(&self) -> Vec<LocaleId> {
        let mut set = BTreeMap::new();
        for key in self.messages.keys() {
            set.insert(key.locale.clone(), ());
        }
        set.into_keys().collect()
    }

    /// 是否存在精确键。
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
