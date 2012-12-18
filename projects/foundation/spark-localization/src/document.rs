//! VON / JSON 共用的作者文档中间模型。
//!
//! 源格式可以是 Oak VON 或 JSON，但必须反序列化到同一结构，再编译为消息 IR。

use std::{collections::BTreeMap, sync::Arc};

use crate::{locale::LocaleId, message::NamespaceId};

/// 作者侧消息名（点分路径，如 `menu.continue`）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageName(Arc<str>);

impl MessageName {
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 参数格式提示。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArgumentFormat {
    None,
    Number,
    Currency { currency_code: Arc<str> },
    Percent,
    Date,
    Time,
    DateTime,
    RelativeTime,
    List,
}

/// 消息 AST 节点（构建期糖展开后的明确表示）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageNode {
    Text(Arc<str>),
    Argument { name: Arc<str>, format: ArgumentFormat },
    MessageRef { name: MessageName, attribute: Option<Arc<str>> },
    Select { argument: Arc<str>, kind: SelectKind, cases: BTreeMap<Arc<str>, Vec<MessageNode>> },
}

/// 选择 / 复数类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectKind {
    /// 显式字符串分支。
    Select,
    /// CLDR 基数复数。
    Cardinal,
    /// CLDR 序数复数。
    Ordinal,
}

/// 单条消息定义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageDefinition {
    /// 简单字符串（构建期可展开为 Text 节点）。
    Text(Arc<str>),
    /// 结构化 pattern。
    Pattern(Vec<MessageNode>),
    /// 顶层 select（常见于复数消息）。
    Select { argument: Arc<str>, kind: SelectKind, cases: BTreeMap<Arc<str>, Vec<MessageNode>> },
}

impl MessageDefinition {
    /// 将模板糖 `"欢迎，{player_name}"` 展开为节点序列。
    ///
    /// 仅识别简单 `{name}`；`{߷name}` 在脚本侧处理，资产侧用结构化 Argument。
    pub fn from_template_sugar(input: &str) -> Self {
        let mut nodes = Vec::new();
        let mut rest = input;
        while let Some(start) = rest.find('{') {
            let (head, after) = rest.split_at(start);
            if !head.is_empty() {
                nodes.push(MessageNode::Text(Arc::from(head)));
            }
            let Some(end) = after.find('}')
            else {
                nodes.push(MessageNode::Text(Arc::from(after)));
                rest = "";
                break;
            };
            let name = after[1..end].trim();
            if name.is_empty() {
                nodes.push(MessageNode::Text(Arc::from(&after[..=end])));
            }
            else {
                nodes.push(MessageNode::Argument { name: Arc::from(name), format: ArgumentFormat::None });
            }
            rest = &after[end + 1..];
        }
        if !rest.is_empty() {
            nodes.push(MessageNode::Text(Arc::from(rest)));
        }
        if nodes.len() == 1 {
            if let MessageNode::Text(text) = &nodes[0] {
                return Self::Text(text.clone());
            }
        }
        Self::Pattern(nodes)
    }
}

/// VON / JSON 反序列化后的语言包文档。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationDocument {
    pub locale: LocaleId,
    pub namespace: NamespaceId,
    pub messages: BTreeMap<MessageName, MessageDefinition>,
}

impl LocalizationDocument {
    pub fn new(locale: LocaleId, namespace: impl Into<Arc<str>>) -> Self {
        Self { locale, namespace: NamespaceId::new(namespace), messages: BTreeMap::new() }
    }

    pub fn insert(&mut self, name: impl Into<Arc<str>>, definition: MessageDefinition) {
        self.messages.insert(MessageName::new(name), definition);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_sugar_splits_arguments() {
        let def = MessageDefinition::from_template_sugar("欢迎回来，{player_name}");
        match def {
            MessageDefinition::Pattern(nodes) => {
                assert_eq!(nodes.len(), 2);
                assert!(matches!(&nodes[0], MessageNode::Text(t) if t.as_ref() == "欢迎回来，"));
                assert!(matches!(
                    &nodes[1],
                    MessageNode::Argument { name, .. } if name.as_ref() == "player_name"
                ));
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
