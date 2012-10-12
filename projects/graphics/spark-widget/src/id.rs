//! 稳定 Widget 标识。同一逻辑控件跨帧必须得到同一 ID。

use std::hash::{Hash, Hasher};

/// 跨帧稳定的控件 ID。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(u64);

impl WidgetId {
    pub const NONE: Self = Self(0);

    pub fn raw(self) -> u64 {
        self.0
    }

    pub fn is_none(self) -> bool {
        self.0 == 0
    }

    /// 由任意可哈希盐生成。
    pub fn from_hashable(value: impl Hash) -> Self {
        let mut hasher = FnHasher(0xcbf29ce484222325);
        value.hash(&mut hasher);
        Self(hasher.0.max(1))
    }

    pub fn with_child(self, child: impl Hash) -> Self {
        let mut hasher = FnHasher(self.0 ^ 0x9e3779b97f4a7c15);
        child.hash(&mut hasher);
        Self(hasher.0.max(1))
    }
}

/// 本帧 ID 栈：父作用域 + 自动序号。
#[derive(Debug, Clone, Default)]
pub struct IdStack {
    stack: Vec<u64>,
    auto: u32,
}

impl IdStack {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn peek(&self) -> WidgetId {
        WidgetId(self.stack.last().copied().unwrap_or(1).max(1))
    }

    pub fn push_hashable(&mut self, salt: impl Hash) -> WidgetId {
        let parent = self.stack.last().copied().unwrap_or(0xcbf29ce484222325);
        let id = WidgetId(parent).with_child(salt);
        self.stack.push(id.0);
        self.auto = 0;
        id
    }

    pub fn push_auto(&mut self) -> WidgetId {
        let n = self.auto;
        self.auto = self.auto.wrapping_add(1);
        self.push_hashable(n)
    }

    pub fn pop(&mut self) {
        let _ = self.stack.pop();
    }
}

/// 轻量 FNV-1a 64，避免拉 `std::collections::hash_map::DefaultHasher` 的平台差异。
struct FnHasher(u64);

impl Hasher for FnHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_path_same_id() {
        let mut a = IdStack::new();
        a.push_hashable("inventory");
        let slot = a.push_hashable(3u32);
        a.pop();
        a.pop();
        let mut b = IdStack::new();
        b.push_hashable("inventory");
        let again = b.push_hashable(3u32);
        assert_eq!(slot, again);
    }
}
