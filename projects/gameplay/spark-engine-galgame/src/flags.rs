//! 旗标 / 变量表。

use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct FlagStore {
    values: HashMap<String, f64>,
}

impl FlagStore {
    pub fn set(&mut self, key: impl Into<String>, value: f64) {
        self.values.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> f64 {
        self.values.get(key).copied().unwrap_or(0.0)
    }

    pub fn is_truthy(&self, key: &str) -> bool {
        self.get(key) != 0.0
    }
}
