//! Spark **资源框架**（无游戏资产格式）。
//!
//! 提供 [`AssetId`]、内存缓存、[`AssetLoader`]、热重载事件、[`HotReloadWatch`] 轮询，
//! 以及旁车 [`.meta`](meta)（VON）持久化身份与路径优先的 [`AssetRef`]。
//! 具体纹理/音频解码由其它 crate / 游戏仓实现加载器。

#![forbid(missing_docs)]
mod cache;
mod handle;
mod hot_reload;
mod index;
mod loader;
mod meta;
mod reference;
mod value;

pub use cache::AssetCache;
pub use handle::{AssetId, AssetKey};
pub use hot_reload::HotReloadWatch;
pub use index::{AssetIndex, AssetIndexError};
pub use loader::{AssetLoader, BytesLoader, LoadError, ReloadEvent};
pub use meta::{ASSET_META_FORMAT, AssetMeta, AssetMetaError, AssetMetaStore};
pub use reference::AssetRef;
pub use value::MetaValue;

use spark_types::{ErrorArgs, SparkError};

/// 资源层统一错误。自然语言不在此生成；`Display` 委派到内层稳定码。
#[derive(Debug)]
pub enum AssetError {
    /// 下层 `spark-types` 错误。
    Spark(SparkError),
    /// 字节/路径加载失败。
    Load(LoadError),
    /// 旁车 `.meta` 读写或身份校验失败。
    Meta(AssetMetaError),
    /// 项目级 GUID ↔ 路径索引失败。
    Index(AssetIndexError),
}

impl AssetError {
    /// 稳定错误码字符串（供本地化与诊断管道）。
    pub fn code(&self) -> String {
        match self {
            Self::Spark(e) => e.code.to_string(),
            Self::Load(e) => e.code().to_string(),
            Self::Meta(e) => e.code().to_string(),
            Self::Index(e) => e.code().to_string(),
        }
    }

    /// 类型化参数（路径、GUID、键等事实），不含面向用户文案。
    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::Spark(e) => e.args.clone(),
            Self::Load(e) => e.args(),
            Self::Meta(e) => e.args(),
            Self::Index(e) => e.args(),
        }
    }
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spark(e) => e.fmt(f),
            Self::Load(e) => e.fmt(f),
            Self::Meta(e) => e.fmt(f),
            Self::Index(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spark(e) => Some(e),
            Self::Load(e) => Some(e),
            Self::Meta(e) => Some(e),
            Self::Index(e) => Some(e),
        }
    }
}

impl From<SparkError> for AssetError {
    fn from(value: SparkError) -> Self {
        Self::Spark(value)
    }
}

impl From<LoadError> for AssetError {
    fn from(value: LoadError) -> Self {
        Self::Load(value)
    }
}

impl From<AssetMetaError> for AssetError {
    fn from(value: AssetMetaError) -> Self {
        Self::Meta(value)
    }
}

impl From<AssetIndexError> for AssetError {
    fn from(value: AssetIndexError) -> Self {
        Self::Index(value)
    }
}
