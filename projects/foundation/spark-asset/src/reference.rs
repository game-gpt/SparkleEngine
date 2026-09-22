//! 资源引用：路径为主、GUID 由工具解析后附着。
//!
//! Agent / Sparkle Script 应书写路径；不要手写或猜测 UUID。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::meta::{AssetMeta, AssetMetaError, AssetMetaStore};

/// 声明式资源引用。
///
/// 序列化时：仅有路径时写成字符串；带 GUID 时写成对象，便于源文件保持可读，
/// 构建产物可携带解析结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssetRef {
    /// 仅路径（Agent / 源文件首选）。
    Path(String),
    /// 路径 + 已解析 GUID（工具写入或缓存用）。
    Resolved {
        /// 资源路径。
        path: String,
        /// 旁车中的持久化身份。
        guid: Uuid,
    },
}

impl AssetRef {
    /// 从路径构造（不读盘）。
    pub fn from_path(path: impl Into<String>) -> Self {
        Self::Path(path.into())
    }

    /// 人类/Agent 使用的路径。
    pub fn path(&self) -> &str {
        match self {
            Self::Path(p) => p.as_str(),
            Self::Resolved { path, .. } => path.as_str(),
        }
    }

    /// 已附着的 GUID（若有）。
    pub fn guid(&self) -> Option<Uuid> {
        match self {
            Self::Path(_) => None,
            Self::Resolved { guid, .. } => Some(*guid),
        }
    }

    /// 读取旁车并附着 GUID；缺失 `.meta` 时返回 [`AssetMetaError::Missing`]。
    pub fn resolve(self) -> Result<Self, AssetMetaError> {
        let path = self.path().to_string();
        let meta = AssetMetaStore::load(&path)?;
        Ok(Self::Resolved { path, guid: meta.guid })
    }

    /// 校验：旁车必须存在；若本引用已带 GUID，则必须与旁车一致。
    pub fn validate(&self) -> Result<AssetMeta, AssetMetaError> {
        let meta = AssetMetaStore::load(self.path())?;
        if let Some(guid) = self.guid() {
            if guid != meta.guid {
                return Err(AssetMetaError::GuidMismatch { asset: PathBuf::from(self.path()), expected: guid, found: meta.guid });
            }
        }
        Ok(meta)
    }
}

impl From<&str> for AssetRef {
    fn from(path: &str) -> Self {
        Self::from_path(path)
    }
}

impl From<String> for AssetRef {
    fn from(path: String) -> Self {
        Self::from_path(path)
    }
}

impl From<&Path> for AssetRef {
    fn from(path: &Path) -> Self {
        Self::from_path(path.to_string_lossy())
    }
}

impl From<PathBuf> for AssetRef {
    fn from(path: PathBuf) -> Self {
        Self::from_path(path.to_string_lossy())
    }
}
