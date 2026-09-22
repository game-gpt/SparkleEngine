//! 可嵌入 `.meta` / Prefab 的开放 VON 值（不绑 `serde_json::Value`）。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// VON 友好的动态值，用于 `settings` / `properties` 等可扩展表。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetaValue {
    /// 布尔。
    Bool(bool),
    /// 整数。
    Int(i64),
    /// 浮点。
    Float(f64),
    /// 字符串。
    String(String),
    /// 数组。
    Array(Vec<MetaValue>),
    /// 表。
    Table(BTreeMap<String, MetaValue>),
}
