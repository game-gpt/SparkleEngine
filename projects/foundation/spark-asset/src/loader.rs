//! 加载器。

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use spark_core::{ErrorArg, ErrorArgs};

use crate::handle::AssetKey;

/// 资源加载失败（稳定码 + 路径事实；`Display` 不输出自然语言）。
#[derive(Debug)]
pub enum LoadError {
    NotFound { key: Arc<str> },
    Io {
        key: Arc<str>,
        cause: std::io::Error,
    },
}

impl LoadError {
    pub fn not_found(key: impl AsRef<str>) -> Self {
        Self::NotFound {
            key: Arc::from(key.as_ref()),
        }
    }

    pub fn io(key: impl AsRef<str>, cause: std::io::Error) -> Self {
        Self::Io {
            key: Arc::from(key.as_ref()),
            cause,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "spark.asset.not_found",
            Self::Io { .. } => "spark.asset.io",
        }
    }

    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::NotFound { key } => {
                ErrorArgs::new().with("key", ErrorArg::AssetKey(Arc::clone(key)))
            }
            Self::Io { key, cause } => ErrorArgs::new()
                .with("key", ErrorArg::AssetKey(Arc::clone(key)))
                .with(
                    "kind",
                    ErrorArg::String(Arc::from(io_kind_token(cause.kind()))),
                ),
        }
    }
}

fn io_kind_token(kind: std::io::ErrorKind) -> &'static str {
    use std::io::ErrorKind::*;
    match kind {
        NotFound => "not_found",
        PermissionDenied => "permission_denied",
        InvalidData => "invalid_data",
        UnexpectedEof => "unexpected_eof",
        AlreadyExists => "already_exists",
        TimedOut => "timed_out",
        _ => "other",
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { cause, .. } => Some(cause),
            Self::NotFound { .. } => None,
        }
    }
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
            return Err(LoadError::not_found(key.as_str()));
        }
        std::fs::read(&path).map_err(|e| LoadError::io(key.as_str(), e))
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
