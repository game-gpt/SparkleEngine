//! 通用数据表：模组注册命名空间键值（自有字符串，不依赖 GC 句柄）。

use std::collections::HashMap;

/// 注册表值（可跨模组 VM 共享，不持有 `GcHandle`）。
#[derive(Debug, Clone, PartialEq)]
pub enum RegValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Entity(u64),
}

impl RegValue {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct DataRegistry {
    tables: HashMap<String, HashMap<String, RegValue>>,
}

impl DataRegistry {
    pub fn set(&mut self, namespace: impl Into<String>, key: impl Into<String>, value: RegValue) {
        self.tables
            .entry(namespace.into())
            .or_default()
            .insert(key.into(), value);
    }

    pub fn get(&self, namespace: &str, key: &str) -> Option<&RegValue> {
        self.tables.get(namespace)?.get(key)
    }

    pub fn remove(&mut self, namespace: &str, key: &str) -> Option<RegValue> {
        self.tables.get_mut(namespace)?.remove(key)
    }

    pub fn keys(&self, namespace: &str) -> Vec<String> {
        self.tables
            .get(namespace)
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn namespaces(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
    }
}
