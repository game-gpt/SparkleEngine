//! 通用属性表（字符串键 → f64）。

use std::collections::HashMap;

/// 稀疏属性表：键由游戏仓约定（如 `hp` / `atk`），缺省读为 `0.0`。
#[derive(Debug, Clone, Default)]
pub struct StatSheet {
    values: HashMap<String, f64>,
}

impl StatSheet {
    /// 写入属性；覆盖旧值。
    pub fn set(&mut self, key: impl Into<String>, value: f64) {
        self.values.insert(key.into(), value);
    }

    /// 读取属性；不存在返回 `0.0`。
    pub fn get(&self, key: &str) -> f64 {
        self.values.get(key).copied().unwrap_or(0.0)
    }

    /// 在当前值上加 `delta`（不存在则从 0 起算）。
    pub fn add(&mut self, key: &str, delta: f64) {
        let v = self.get(key) + delta;
        self.set(key, v);
    }

    /// 已写入的键迭代器。
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }
}
