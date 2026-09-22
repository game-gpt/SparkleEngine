//! 加载器。

use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use spark_types::{ErrorArg, ErrorArgs};

use crate::handle::AssetKey;

/// 资源加载失败（稳定码 + 路径事实；`Display` 不输出自然语言）。
#[derive(Debug)]
pub enum LoadError {
    /// 解析后的路径不是普通文件，或宿主明确报“未找到”。
    NotFound {
        /// 请求时的资源键。
        key: Arc<str>,
    },
    /// 打开/读取过程中的 IO 失败（非“单纯不存在”时也可出现）。
    Io {
        /// 请求时的资源键。
        key: Arc<str>,
        /// 底层 IO 错误（经 `Error::source` 暴露）。
        cause: std::io::Error,
    },
}

impl LoadError {
    /// 构造 [`LoadError::NotFound`]。
    pub fn not_found(key: impl AsRef<str>) -> Self {
        Self::NotFound { key: Arc::from(key.as_ref()) }
    }

    /// 构造 [`LoadError::Io`]。
    pub fn io(key: impl AsRef<str>, cause: std::io::Error) -> Self {
        Self::Io { key: Arc::from(key.as_ref()), cause }
    }

    /// 稳定错误码：`spark.asset.not_found` / `spark.asset.io`。
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "spark.asset.not_found",
            Self::Io { .. } => "spark.asset.io",
        }
    }

    /// 类型化参数：`key`，以及 IO 时的 `kind` token。
    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::NotFound { key } => ErrorArgs::new().with("key", ErrorArg::AssetKey(Arc::clone(key))),
            Self::Io { key, cause } => ErrorArgs::new()
                .with("key", ErrorArg::AssetKey(Arc::clone(key)))
                .with("kind", ErrorArg::String(Arc::from(io_kind_token(cause.kind())))),
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

/// 按 [`AssetKey`] 产出原始字节的加载后端。
///
/// 实现须 `Send + Sync`；缓存与热重载通过 `&dyn AssetLoader` 调用。
pub trait AssetLoader: Send + Sync {
    /// 读取键对应的完整字节；失败返回 [`LoadError`]（勿 panic）。
    fn load(&self, key: &AssetKey) -> Result<Vec<u8>, LoadError>;
}

/// 相对根目录读文件的字节加载器。
#[derive(Debug, Clone)]
pub struct BytesLoader {
    root: PathBuf,
}

impl BytesLoader {
    /// 以 `root` 为资源根；键为相对路径片段。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// `root.join(key)`，不做路径穿越规范化。
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

/// 一次热重载通知：逻辑键 + 实际触达路径。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadEvent {
    /// 缓存中的资源键。
    pub key: AssetKey,
    /// 文件路径（轮询监视时可覆写为真实磁盘路径）。
    pub path: PathBuf,
}

impl ReloadEvent {
    /// 由键与路径构造；路径经 `AsRef<Path>` 转成拥有型 [`PathBuf`]。
    pub fn new(key: impl Into<AssetKey>, path: impl AsRef<Path>) -> Self {
        Self { key: key.into(), path: path.as_ref().to_path_buf() }
    }
}
