//! 类型化错误参数（禁止预先拼好的自然语言句子）。

use std::{collections::BTreeMap, sync::Arc};

/// 单个错误参数值。
///
/// 渲染边界按变体选择插值格式；不得把面向用户的整句塞进 [`ErrorArg::String`]。
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorArg {
    /// 有符号整数（索引、偏移、错误码附属数值等）。
    Integer(i64),
    /// 无符号整数（长度、容量、实体计数等）。
    Unsigned(u64),
    /// 浮点（尺寸、比例、时长等；单位由参数名约定）。
    Float(f64),
    /// 非用户句子的符号/原文（第三方 opaque、标识符等）。
    String(Arc<str>),
    /// 逻辑资源键。
    AssetKey(Arc<str>),
    /// 逻辑路径（非本机盘符权威）。
    Path(Arc<str>),
    /// 字节码 / 指令操作码。
    Opcode(u8),
    /// ECS 实体原始位（避免 diagnostics 依赖 spark-ecs）。
    EntityBits(u64),
    /// 类型名（反射或调试用，非用户文案）。
    TypeName(Arc<str>),
    /// 布尔标志。
    Bool(bool),
    /// 源码字节范围。
    Span(crate::SourceSpan),
}

/// 具名参数表（有序，便于测试与序列化）。
///
/// 键为稳定英文标识（如 `path`、`width`）；值不得含本地化句子。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ErrorArgs {
    values: BTreeMap<Arc<str>, ErrorArg>,
}

impl ErrorArgs {
    /// 空参数表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 插入或覆盖具名参数；返回 `self` 便于链式调用。
    pub fn insert(&mut self, name: impl Into<Arc<str>>, value: ErrorArg) -> &mut Self {
        self.values.insert(name.into(), value);
        self
    }

    /// 消费式插入，便于建造模式。
    pub fn with(mut self, name: impl Into<Arc<str>>, value: ErrorArg) -> Self {
        self.insert(name, value);
        self
    }

    /// 按名查找；不存在则 `None`。
    pub fn get(&self, name: &str) -> Option<&ErrorArg> {
        self.values.get(name)
    }

    /// 按键名升序迭代 `(name, value)`。
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ErrorArg)> {
        self.values.iter().map(|(k, v)| (k.as_ref(), v))
    }

    /// 是否无任何参数。
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// 参数个数。
    pub fn len(&self) -> usize {
        self.values.len()
    }
}
