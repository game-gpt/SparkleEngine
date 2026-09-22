//! 通用数据表：模组注册命名空间键值（自有字符串，不依赖 GC 句柄）。

use std::collections::HashMap;

/// 注册表值（可跨模组 VM 共享，不持有 `GcHandle`）。
#[derive(Debug, Clone, PartialEq)]
pub enum RegValue {
    /// 空值 / 占位。
    Null,
    /// 布尔。
    Bool(bool),
    /// 双精度数字（脚本侧统一数值）。
    Number(f64),
    /// 自有字符串（非 GC）。
    String(String),
    /// 实体位模式句柄（与 ECS `Entity::to_bits` 一致）。
    Entity(u64),
}

impl RegValue {
    /// 尝试视为数字：`Number` 原样；`Bool` 映射为 0/1；其余 `None`。
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    /// 仅当为 [`Self::String`] 时返回切片。
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}

/// 分命名空间的键值表；游戏层解释语义，引擎只做存取。
#[derive(Debug, Default)]
pub struct DataRegistry {
    tables: HashMap<String, HashMap<String, RegValue>>,
}

impl DataRegistry {
    /// 写入（覆盖同键）；命名空间不存在时自动创建。
    pub fn set(&mut self, namespace: impl Into<String>, key: impl Into<String>, value: RegValue) {
        self.tables.entry(namespace.into()).or_default().insert(key.into(), value);
    }

    /// 读取；命名空间或键不存在时返回 `None`。
    pub fn get(&self, namespace: &str, key: &str) -> Option<&RegValue> {
        self.tables.get(namespace)?.get(key)
    }

    /// 移除并返回旧值；键不存在时 `None`。
    pub fn remove(&mut self, namespace: &str, key: &str) -> Option<RegValue> {
        self.tables.get_mut(namespace)?.remove(key)
    }

    /// 某命名空间下全部键名（无序）；命名空间不存在时为空。
    pub fn keys(&self, namespace: &str) -> Vec<String> {
        self.tables.get(namespace).map(|t| t.keys().cloned().collect()).unwrap_or_default()
    }

    /// 全部命名空间名（无序）。
    pub fn namespaces(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
    }
}
