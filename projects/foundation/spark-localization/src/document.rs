//! VON / JSON 共用的作者文档中间模型。
//!
//! 源格式可以是 Oak VON 或 JSON，但必须反序列化到同一结构，再编译为消息 IR。

use std::{collections::BTreeMap, sync::Arc};

use crate::{locale::LocaleId, message::NamespaceId};

/// 作者侧消息名（点分路径，如 `menu.continue`）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageName(Arc<str>);

impl MessageName {
    /// 由任意可转成 `Arc<str>` 的名字构造。
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self(name.into())
    }

    /// 原始字符串视图。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 参数格式提示。
///
/// 求值器可按 Locale 文化规则渲染；未知格式不得回退为 HTML。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArgumentFormat {
    /// 无额外格式，按值类型默认展示。
    None,
    /// 数值（整数 / 定点小数）。
    Number,
    /// 货币金额。
    Currency {
        /// ISO 4217 货币码（如 `CNY`）。
        currency_code: Arc<str>,
    },
    /// 百分比。
    Percent,
    /// 仅日期部分。
    Date,
    /// 仅时间部分。
    Time,
    /// 日期 + 时间。
    DateTime,
    /// 相对时间（如「3 分钟前」语义由渲染层决定）。
    RelativeTime,
    /// 列表连接。
    List,
}

/// 消息 AST 节点（构建期糖展开后的明确表示）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageNode {
    /// 字面文本片段。
    Text(Arc<str>),
    /// 具名参数插值。
    Argument {
        /// 参数名，对应 [`crate::MessageArgs`] 键。
        name: Arc<str>,
        /// 展示格式提示。
        format: ArgumentFormat,
    },
    /// 引用同命名空间内另一条消息。
    MessageRef {
        /// 被引用消息名。
        name: MessageName,
        /// 可选属性名（Fluent 风格；当前求值可仅作诊断占位）。
        attribute: Option<Arc<str>>,
    },
    /// 嵌套选择 / 复数分支。
    Select {
        /// 驱动分支的参数名。
        argument: Arc<str>,
        /// 分支语义。
        kind: SelectKind,
        /// 分支键 → 子节点；检查要求含 `other`。
        cases: BTreeMap<Arc<str>, Vec<MessageNode>>,
    },
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
    Select {
        /// 驱动分支的参数名。
        argument: Arc<str>,
        /// 分支语义。
        kind: SelectKind,
        /// 分支键 → 子节点；检查要求含 `other`。
        cases: BTreeMap<Arc<str>, Vec<MessageNode>>,
    },
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
    /// 文档所属 Locale。
    pub locale: LocaleId,
    /// 消息命名空间。
    pub namespace: NamespaceId,
    /// 消息名 → 定义；键有序便于差分与快照比对。
    pub messages: BTreeMap<MessageName, MessageDefinition>,
}

impl LocalizationDocument {
    /// 构造空文档。
    pub fn new(locale: LocaleId, namespace: impl Into<Arc<str>>) -> Self {
        Self { locale, namespace: NamespaceId::new(namespace), messages: BTreeMap::new() }
    }

    /// 插入或覆盖一条消息定义。
    pub fn insert(&mut self, name: impl Into<Arc<str>>, definition: MessageDefinition) {
        self.messages.insert(MessageName::new(name), definition);
    }
}
