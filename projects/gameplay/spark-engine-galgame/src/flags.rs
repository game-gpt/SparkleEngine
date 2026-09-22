//! 旗标 / 变量表。
//!
//! 键为字符串，值为 `f64`。未写入的键读取为 `0.0`；[`FlagStore::is_truthy`] 以「非零」为真。

use std::collections::HashMap;

/// 剧本侧旗标表（与 [`crate::script::ScriptPlayer`] 选项副作用共用）。
#[derive(Debug, Default, Clone)]
pub struct FlagStore {
    values: HashMap<String, f64>,
}

impl FlagStore {
    /// 写入或覆盖键值。
    pub fn set(&mut self, key: impl Into<String>, value: f64) {
        self.values.insert(key.into(), value);
    }

    /// 读取键值；缺失时返回 `0.0`。
    pub fn get(&self, key: &str) -> f64 {
        self.values.get(key).copied().unwrap_or(0.0)
    }

    /// 非零为真（含负值）；缺失键视为假。
    pub fn is_truthy(&self, key: &str) -> bool {
        self.get(key) != 0.0
    }
}
