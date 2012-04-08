//! 通用属性表（字符串键 → f64）。

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct StatSheet {
    values: HashMap<String, f64>,
}

impl StatSheet {
    pub fn set(&mut self, key: impl Into<String>, value: f64) {
        self.values.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> f64 {
        self.values.get(key).copied().unwrap_or(0.0)
    }

    pub fn add(&mut self, key: &str, delta: f64) {
        let v = self.get(key) + delta;
        self.set(key, v);
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }
}
