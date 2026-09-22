//! 资源键与句柄。

use std::path::{Path, PathBuf};

/// 逻辑资源键（通常为相对根目录的路径字符串）。
///
/// 不变式：比较与哈希按完整字符串；不在此处规范化分隔符或大小写。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetKey(
    /// 键正文；构造后视为不可变身份字符串。
    pub String,
);

impl AssetKey {
    /// 从任意可转成 `String` 的值构造。
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// 借用键正文。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AssetKey {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<PathBuf> for AssetKey {
    fn from(p: PathBuf) -> Self {
        Self::new(p.to_string_lossy())
    }
}

impl From<&Path> for AssetKey {
    fn from(p: &Path) -> Self {
        Self::new(p.to_string_lossy())
    }
}

/// 缓存内稳定句柄（不透明）。
///
/// 槽位可在 [`crate::AssetCache::remove`] 后复用；持有方应在移除后丢弃旧 id。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(
    /// 缓存槽索引；`0` 起，与 `entries` 下标对应。
    pub u32,
);
