//! 资源旁车 `.meta`：持久化身份（GUID）与导入器配置。
//!
//! Agent / 编辑脚本应通过路径引用资源；UUID v7 仅由本模块与上层工具生成。
//! 缺失的 `.meta` **不会**在 `load` 时静默换发新 GUID（避免破坏既有引用）。

use std::{
    collections::BTreeMap,
    fmt,
    fs,
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use spark_types::{ErrorArg, ErrorArgs};
use uuid::Uuid;

/// 当前 `.meta` JSON 格式版本。
pub const ASSET_META_FORMAT: u32 = 1;

/// 旁车元数据正文。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetMeta {
    /// 格式版本（字段名 `format`，与 JSON 对齐）。
    pub format: u32,
    /// 资源持久化身份（UUID v7）。
    pub guid: Uuid,
    /// 资源类型提示（如 `image`、`prefab`），可选。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// 导入器标识，可选。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importer: Option<String>,
    /// 导入器逻辑版本，可选。
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "importerVersion")]
    pub importer_version: Option<u32>,
    /// 源文件内容哈希（如 `sha256:…`），可选。
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "sourceHash")]
    pub source_hash: Option<String>,
    /// 导入器设置（非玩法权威）。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub settings: BTreeMap<String, serde_json::Value>,
    /// 可扩展属性（展示名、标签等不得影响身份）。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, serde_json::Value>,
}

impl AssetMeta {
    /// 新建身份：UUID v7，`format = `[`ASSET_META_FORMAT`]。
    pub fn new_identity() -> Self {
        Self {
            format: ASSET_META_FORMAT,
            guid: Uuid::now_v7(),
            kind: None,
            importer: None,
            importer_version: None,
            source_hash: None,
            settings: BTreeMap::new(),
            properties: BTreeMap::new(),
        }
    }
}

/// `.meta` 读写错误（稳定码；`Display` 不输出自然语言）。
#[derive(Debug)]
pub enum AssetMetaError {
    /// 旁车不存在；调用方应报告诊断，勿静默 `create` 覆盖旧引用。
    Missing {
        /// 资源路径（非旁车路径）。
        asset: PathBuf,
    },
    /// 旁车已存在，拒绝覆盖身份。
    AlreadyExists {
        /// 已存在的旁车路径。
        meta: PathBuf,
    },
    /// 引用上的 GUID 与旁车不一致。
    GuidMismatch {
        /// 资源路径。
        asset: PathBuf,
        /// 引用中的 GUID。
        expected: Uuid,
        /// 旁车中的 GUID。
        found: Uuid,
    },
    /// 读写 IO 失败。
    Io {
        /// 相关路径。
        path: PathBuf,
        /// 底层错误。
        cause: io::Error,
    },
    /// JSON 解析/序列化失败。
    Parse {
        /// 相关路径。
        path: PathBuf,
        /// 细节（非面向用户文案权威）。
        detail: String,
    },
}

impl AssetMetaError {
    /// 稳定错误码。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Missing { .. } => "spark.asset.meta_missing",
            Self::AlreadyExists { .. } => "spark.asset.meta_already_exists",
            Self::GuidMismatch { .. } => "spark.asset.guid_mismatch",
            Self::Io { .. } => "spark.asset.meta_io",
            Self::Parse { .. } => "spark.asset.meta_parse",
        }
    }

    /// 类型化参数（供本地化与 Agent 诊断）。
    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::Missing { asset } => {
                ErrorArgs::new().with("path", ErrorArg::String(Arc::from(asset.to_string_lossy().as_ref())))
            }
            Self::AlreadyExists { meta } => {
                ErrorArgs::new().with("path", ErrorArg::String(Arc::from(meta.to_string_lossy().as_ref())))
            }
            Self::GuidMismatch {
                asset,
                expected,
                found,
            } => ErrorArgs::new()
                .with("path", ErrorArg::String(Arc::from(asset.to_string_lossy().as_ref())))
                .with("expected", ErrorArg::String(Arc::from(expected.to_string())))
                .with("found", ErrorArg::String(Arc::from(found.to_string()))),
            Self::Io { path, cause } => ErrorArgs::new()
                .with("path", ErrorArg::String(Arc::from(path.to_string_lossy().as_ref())))
                .with("kind", ErrorArg::String(Arc::from(io_kind_token(cause.kind())))),
            Self::Parse { path, detail } => ErrorArgs::new()
                .with("path", ErrorArg::String(Arc::from(path.to_string_lossy().as_ref())))
                .with("detail", ErrorArg::String(Arc::from(detail.as_str()))),
        }
    }
}

fn io_kind_token(kind: io::ErrorKind) -> &'static str {
    use io::ErrorKind::*;
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

impl fmt::Display for AssetMetaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for AssetMetaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { cause, .. } => Some(cause),
            _ => None,
        }
    }
}

/// 旁车 `.meta` 路径与读写。
pub struct AssetMetaStore;

impl AssetMetaStore {
    /// `assets/image.png` → `assets/image.png.meta`。
    pub fn path(asset: impl AsRef<Path>) -> PathBuf {
        let asset = asset.as_ref();
        let mut os = asset.as_os_str().to_owned();
        os.push(".meta");
        PathBuf::from(os)
    }

    /// 读取已有旁车；缺失时返回 [`AssetMetaError::Missing`]（不生成新 GUID）。
    pub fn load(asset: impl AsRef<Path>) -> Result<AssetMeta, AssetMetaError> {
        let asset = asset.as_ref();
        let meta_path = Self::path(asset);
        match fs::read_to_string(&meta_path) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| AssetMetaError::Parse {
                path: meta_path,
                detail: e.to_string(),
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Err(AssetMetaError::Missing {
                asset: asset.to_path_buf(),
            }),
            Err(e) => Err(AssetMetaError::Io {
                path: meta_path,
                cause: e,
            }),
        }
    }

    /// 写入旁车（格式化 JSON，便于 Git 与人工检查）。
    pub fn save(asset: impl AsRef<Path>, meta: &AssetMeta) -> Result<(), AssetMetaError> {
        let meta_path = Self::path(asset.as_ref());
        if let Some(parent) = meta_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| AssetMetaError::Io {
                    path: parent.to_path_buf(),
                    cause: e,
                })?;
            }
        }
        let text = serde_json::to_string_pretty(meta).map_err(|e| AssetMetaError::Parse {
            path: meta_path.clone(),
            detail: e.to_string(),
        })?;
        fs::write(&meta_path, format!("{text}\n")).map_err(|e| AssetMetaError::Io {
            path: meta_path,
            cause: e,
        })
    }

    /// 为**新资源**生成身份并写入；若旁车已存在则失败，避免静默换 GUID。
    pub fn create(asset: impl AsRef<Path>) -> Result<AssetMeta, AssetMetaError> {
        let asset = asset.as_ref();
        let meta_path = Self::path(asset);
        if meta_path.is_file() {
            return Err(AssetMetaError::AlreadyExists { meta: meta_path });
        }
        let meta = AssetMeta::new_identity();
        Self::save(asset, &meta)?;
        Ok(meta)
    }

    /// 工具侧首次登记：有则加载，无则 [`create`]。
    ///
    /// 仅用于明确的新建/导入流程；**不要**用它“修复”丢失的 `.meta`（应报告
    /// [`AssetMetaError::Missing`] 并由人工或带预览的 repair 决定是否换发身份）。
    pub fn load_or_create(asset: impl AsRef<Path>) -> Result<AssetMeta, AssetMetaError> {
        match Self::load(asset.as_ref()) {
            Ok(meta) => Ok(meta),
            Err(AssetMetaError::Missing { .. }) => Self::create(asset),
            Err(e) => Err(e),
        }
    }
}
