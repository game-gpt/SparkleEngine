//! Prefab 作为资源登记：写 `.prefab` 并确保旁车 `.meta`（kind = prefab）。

use std::path::Path;

use spark_asset::{AssetMeta, AssetMetaError, AssetMetaStore};

use crate::document::PrefabDocument;
use crate::error::PrefabError;

/// 登记失败：旁车或 Prefab IO。
#[derive(Debug)]
pub enum PrefabRegisterError {
    /// `.meta` 层错误。
    Meta(AssetMetaError),
    /// Prefab 文档错误。
    Prefab(PrefabError),
}

impl PrefabRegisterError {
    /// 稳定错误码。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Meta(e) => e.code(),
            Self::Prefab(e) => e.code(),
        }
    }
}

impl std::fmt::Display for PrefabRegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for PrefabRegisterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Meta(e) => Some(e),
            Self::Prefab(e) => Some(e),
        }
    }
}

impl From<AssetMetaError> for PrefabRegisterError {
    fn from(value: AssetMetaError) -> Self {
        Self::Meta(value)
    }
}

impl From<PrefabError> for PrefabRegisterError {
    fn from(value: PrefabError) -> Self {
        Self::Prefab(value)
    }
}

/// 保存 Prefab，并确保旁车身份（新建或复用已有 GUID）。
///
/// - 校验文档结构
/// - 写入 `.prefab`
/// - 无 `.meta` 则 `create` 并写入 `kind = "prefab"`
/// - 已有 `.meta` 则加载，必要时补上 `kind`
pub fn save_registered(doc: &PrefabDocument, path: impl AsRef<Path>) -> Result<AssetMeta, PrefabRegisterError> {
    let path = path.as_ref();
    doc.validate()?;
    doc.save(path)?;
    let meta = match AssetMetaStore::load(path) {
        Ok(mut meta) => {
            if meta.kind.as_deref() != Some("prefab") {
                meta.kind = Some("prefab".into());
                AssetMetaStore::save(path, &meta)?;
            }
            meta
        }
        Err(AssetMetaError::Missing { .. }) => {
            let mut meta = AssetMetaStore::create(path)?;
            meta.kind = Some("prefab".into());
            AssetMetaStore::save(path, &meta)?;
            meta
        }
        Err(e) => return Err(e.into()),
    };
    Ok(meta)
}
