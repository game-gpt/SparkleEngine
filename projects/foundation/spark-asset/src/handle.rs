//! 资源键与句柄。

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetKey(pub String);

impl AssetKey {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(pub u32);
