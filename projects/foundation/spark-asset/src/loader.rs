//! 加载器。

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::handle::AssetKey;

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("资源未找到：{0}")]
    NotFound(String),
    #[error("IO：{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Message(String),
}

pub trait AssetLoader: Send + Sync {
    fn load(&self, key: &AssetKey) -> Result<Vec<u8>, LoadError>;
}

/// 相对根目录读文件的字节加载器。
#[derive(Debug, Clone)]
pub struct BytesLoader {
    root: PathBuf,
}

impl BytesLoader {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn resolve(&self, key: &AssetKey) -> PathBuf {
        self.root.join(key.as_str())
    }
}

impl AssetLoader for BytesLoader {
    fn load(&self, key: &AssetKey) -> Result<Vec<u8>, LoadError> {
        let path = self.resolve(key);
        if !path.is_file() {
            return Err(LoadError::NotFound(path.display().to_string()));
        }
        Ok(std::fs::read(&path)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadEvent {
    pub key: AssetKey,
    pub path: PathBuf,
}

impl ReloadEvent {
    pub fn new(key: impl Into<AssetKey>, path: impl AsRef<Path>) -> Self {
        Self {
            key: key.into(),
            path: path.as_ref().to_path_buf(),
        }
    }
}
