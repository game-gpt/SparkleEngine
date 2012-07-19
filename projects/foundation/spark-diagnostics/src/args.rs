//! 类型化错误参数（禁止预先拼好的自然语言句子）。

use std::collections::BTreeMap;
use std::sync::Arc;

/// 单个错误参数值。
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorArg {
    Integer(i64),
    Unsigned(u64),
    Float(f64),
    /// 非用户句子的符号/原文（第三方 opaque、标识符等）。
    String(Arc<str>),
    /// 逻辑资源键。
    AssetKey(Arc<str>),
    /// 逻辑路径（非本机盘符权威）。
    Path(Arc<str>),
    Opcode(u8),
    /// ECS 实体原始位（避免 diagnostics 依赖 spark-ecs）。
    EntityBits(u64),
    TypeName(Arc<str>),
    Bool(bool),
    /// 源码字节范围。
    Span(crate::SourceSpan),
}

/// 具名参数表（有序，便于测试与序列化）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ErrorArgs {
    values: BTreeMap<Arc<str>, ErrorArg>,
}

impl ErrorArgs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<Arc<str>>, value: ErrorArg) -> &mut Self {
        self.values.insert(name.into(), value);
        self
    }

    pub fn with(mut self, name: impl Into<Arc<str>>, value: ErrorArg) -> Self {
        self.insert(name, value);
        self
    }

    pub fn get(&self, name: &str) -> Option<&ErrorArg> {
        self.values.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &ErrorArg)> {
        self.values.iter().map(|(k, v)| (k.as_ref(), v))
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }
}
