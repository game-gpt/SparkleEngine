//! 项目级资源身份索引：GUID ↔ 路径。
//!
//! 由工具维护；Agent 只操作路径。重命名时移动源文件与旁车，索引随之更新。

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use spark_types::{ErrorArg, ErrorArgs};
use uuid::Uuid;

use crate::meta::{AssetMetaError, AssetMetaStore};

/// 扫描/索引错误。
#[derive(Debug)]
pub enum AssetIndexError {
    /// 旁车层错误。
    Meta(AssetMetaError),
    /// 同一 GUID 出现在多条旁车。
    DuplicateGuid {
        /// 冲突 GUID。
        guid: Uuid,
        /// 已登记路径。
        first: PathBuf,
        /// 再次出现的路径。
        second: PathBuf,
    },
    /// 目录遍历失败。
    Walk {
        /// 起点。
        root: PathBuf,
        /// 细节。
        detail: String,
    },
}

impl AssetIndexError {
    /// 稳定错误码。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Meta(e) => e.code(),
            Self::DuplicateGuid { .. } => "spark.asset.index_duplicate_guid",
            Self::Walk { .. } => "spark.asset.index_walk",
        }
    }

    /// 类型化参数。
    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::Meta(e) => e.args(),
            Self::DuplicateGuid { guid, first, second } => ErrorArgs::new()
                .with("guid", ErrorArg::String(Arc::from(guid.to_string())))
                .with("first", ErrorArg::String(Arc::from(first.to_string_lossy().as_ref())))
                .with("second", ErrorArg::String(Arc::from(second.to_string_lossy().as_ref()))),
            Self::Walk { root, detail } => ErrorArgs::new()
                .with("path", ErrorArg::String(Arc::from(root.to_string_lossy().as_ref())))
                .with("detail", ErrorArg::String(Arc::from(detail.as_str()))),
        }
    }
}

impl std::fmt::Display for AssetIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for AssetIndexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Meta(e) => Some(e),
            _ => None,
        }
    }
}

impl From<AssetMetaError> for AssetIndexError {
    fn from(value: AssetMetaError) -> Self {
        Self::Meta(value)
    }
}

/// GUID ↔ 路径双向索引（内存）。
#[derive(Debug, Default, Clone)]
pub struct AssetIndex {
    by_guid: BTreeMap<Uuid, PathBuf>,
    by_path: BTreeMap<PathBuf, Uuid>,
}

impl AssetIndex {
    /// 空索引。
    pub fn new() -> Self {
        Self::default()
    }

    /// 条目数。
    pub fn len(&self) -> usize {
        self.by_guid.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.by_guid.is_empty()
    }

    /// 按 GUID 查路径。
    pub fn path_of(&self, guid: Uuid) -> Option<&Path> {
        self.by_guid.get(&guid).map(PathBuf::as_path)
    }

    /// 按路径查 GUID。
    pub fn guid_of(&self, path: impl AsRef<Path>) -> Option<Uuid> {
        self.by_path.get(path.as_ref()).copied()
    }

    /// 登记一条已存在旁车的资源；GUID 冲突则失败。
    pub fn insert(&mut self, asset: impl AsRef<Path>, guid: Uuid) -> Result<(), AssetIndexError> {
        let asset = asset.as_ref().to_path_buf();
        if let Some(existing) = self.by_guid.get(&guid) {
            if existing != &asset {
                return Err(AssetIndexError::DuplicateGuid {
                    guid,
                    first: existing.clone(),
                    second: asset,
                });
            }
        }
        if let Some(old_guid) = self.by_path.insert(asset.clone(), guid) {
            if old_guid != guid {
                self.by_guid.remove(&old_guid);
            }
        }
        self.by_guid.insert(guid, asset);
        Ok(())
    }

    /// 从单个资源的 `.meta` 读入并登记。
    pub fn register(&mut self, asset: impl AsRef<Path>) -> Result<Uuid, AssetIndexError> {
        let asset = asset.as_ref();
        let meta = AssetMetaStore::load(asset)?;
        self.insert(asset, meta.guid)?;
        Ok(meta.guid)
    }

    /// 递归扫描 `root` 下所有 `*.meta`，反推资源路径并登记。
    pub fn scan(root: impl AsRef<Path>) -> Result<Self, AssetIndexError> {
        let root = root.as_ref();
        let mut index = Self::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let rd = fs::read_dir(&dir).map_err(|e| AssetIndexError::Walk {
                root: dir.clone(),
                detail: e.to_string(),
            })?;
            for entry in rd {
                let entry = entry.map_err(|e| AssetIndexError::Walk {
                    root: dir.clone(),
                    detail: e.to_string(),
                })?;
                let path = entry.path();
                let ft = entry.file_type().map_err(|e| AssetIndexError::Walk {
                    root: path.clone(),
                    detail: e.to_string(),
                })?;
                if ft.is_dir() {
                    stack.push(path);
                    continue;
                }
                if !ft.is_file() {
                    continue;
                }
                let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                    continue;
                };
                if !name.ends_with(".meta") {
                    continue;
                }
                let asset = path.with_file_name(&name[..name.len() - ".meta".len()]);
                index.register(&asset)?;
            }
        }
        Ok(index)
    }

    /// 移动资源文件与旁车，并更新索引中的路径。
    ///
    /// 不改 GUID。目标旁车已存在则失败。
    pub fn rename(&mut self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<(), AssetIndexError> {
        let from = from.as_ref();
        let to = to.as_ref();
        let guid = self.guid_of(from).ok_or_else(|| AssetMetaError::Missing {
            asset: from.to_path_buf(),
        })?;
        let from_meta = AssetMetaStore::path(from);
        let to_meta = AssetMetaStore::path(to);
        if to_meta.is_file() {
            return Err(AssetMetaError::AlreadyExists { meta: to_meta }.into());
        }
        if let Some(parent) = to.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| AssetMetaError::Io {
                    path: parent.to_path_buf(),
                    cause: e,
                })?;
            }
        }
        if from.is_file() {
            fs::rename(from, to).map_err(|e| AssetMetaError::Io {
                path: from.to_path_buf(),
                cause: e,
            })?;
        }
        if from_meta.is_file() {
            fs::rename(&from_meta, &to_meta).map_err(|e| AssetMetaError::Io {
                path: from_meta,
                cause: e,
            })?;
        }
        self.by_path.remove(from);
        self.by_guid.insert(guid, to.to_path_buf());
        self.by_path.insert(to.to_path_buf(), guid);
        Ok(())
    }
}
