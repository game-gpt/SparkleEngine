//! 消息标识、具名参数与类型化取值。

use std::{collections::BTreeMap, fmt, sync::Arc};

/// 命名空间 ID（构建期可映射为紧凑整数；运行时保留字符串入口）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamespaceId(Arc<str>);

impl NamespaceId {
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 引擎保留前缀。
    pub fn is_spark_reserved(&self) -> bool {
        self.0.as_ref() == "spark" || self.0.starts_with("spark.")
    }
}

impl fmt::Display for NamespaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 命名空间内消息 ID。内建内容可走 `u32` 快路径；动态键走字符串。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MessageId {
    /// 构建期 / 静态绑定生成的紧凑 ID。
    Compact(u32),
    /// 插件、模组与动态查找入口。
    Name(Arc<str>),
}

impl MessageId {
    pub fn compact(id: u32) -> Self {
        Self::Compact(id)
    }

    pub fn name(name: impl Into<Arc<str>>) -> Self {
        Self::Name(name.into())
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compact(id) => write!(f, "#{id}"),
            Self::Name(name) => f.write_str(name),
        }
    }
}

/// 跨命名空间的消息引用。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageRef {
    pub namespace: NamespaceId,
    pub message: MessageId,
}

impl MessageRef {
    pub fn new(namespace: impl Into<Arc<str>>, message: MessageId) -> Self {
        Self { namespace: NamespaceId::new(namespace), message }
    }

    pub fn named(namespace: impl Into<Arc<str>>, message: impl Into<Arc<str>>) -> Self {
        Self::new(namespace, MessageId::name(message))
    }
}

impl fmt::Display for MessageRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.namespace, self.message)
    }
}

/// 可选消息属性（Fluent 风格 attribute；此处仅作 ID，不绑定 Fluent 协议）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttributeId(Arc<str>);

impl AttributeId {
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 语义时长（未格式化）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MessageDuration {
    pub milliseconds: i64,
}

/// 瞬时时间 + 显式时区偏移（分钟，东为正）。
///
/// 禁止依赖宿主本地时区作为隐式真相。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MessageDateTime {
    /// Unix 纪元毫秒。
    pub unix_millis: i64,
    /// 相对 UTC 的分钟偏移。
    pub utc_offset_minutes: i32,
}

/// 十进制数值的定点表示（避免浮点展示歧义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MessageDecimal {
    /// 缩放后的整数部分。
    pub coefficient: i128,
    /// 10 的负指数（小数位数）。
    pub scale: u32,
}

impl MessageDecimal {
    pub fn from_i64(value: i64) -> Self {
        Self { coefficient: i128::from(value), scale: 0 }
    }
}

/// 消息参数值：调用方传语义，不传已格式化展示串。
#[derive(Debug, Clone, PartialEq)]
pub enum MessageValue {
    String(Arc<str>),
    Integer(i64),
    Decimal(MessageDecimal),
    DateTime(MessageDateTime),
    Duration(MessageDuration),
    List(Arc<[MessageValue]>),
    Select(Arc<str>),
}

/// 具名参数表（有序，便于快照与测试比对）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MessageArgs {
    values: BTreeMap<Arc<str>, MessageValue>,
}

impl MessageArgs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<Arc<str>>, value: MessageValue) -> &mut Self {
        self.values.insert(name.into(), value);
        self
    }

    pub fn get(&self, name: &str) -> Option<&MessageValue> {
        self.values.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &MessageValue)> {
        self.values.iter().map(|(k, v)| (k.as_ref(), v))
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_ref_display() {
        let r = MessageRef::named("astracraft", "menu.continue");
        assert_eq!(r.to_string(), "astracraft.menu.continue");
        assert!(!r.namespace.is_spark_reserved());
        assert!(NamespaceId::new("spark.widget").is_spark_reserved());
    }

    #[test]
    fn args_are_named() {
        let mut args = MessageArgs::new();
        args.insert("count", MessageValue::Integer(12));
        assert_eq!(args.get("count"), Some(&MessageValue::Integer(12)));
    }
}
