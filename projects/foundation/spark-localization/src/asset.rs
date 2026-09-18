//! 经 `spark-asset` 装载本地化 JSON 文档并编译为包 / 快照。

use std::fmt;
use std::sync::Arc;

use spark_asset::{AssetKey, AssetLoader, LoadError};

use crate::bundle::LocalizationBundle;
use crate::compile::{CompileError, CompileOptions, compile_documents};
use crate::document::LocalizationDocument;
use crate::json::{JsonError, document_from_json_slice};
use crate::locale::{LocaleRequest, negotiate};
use crate::manifest::LocalizationManifest;
use crate::snapshot::LocaleSnapshot;

/// 资产装载 / 编译错误。
#[derive(Debug)]
pub enum LocaleLoadError {
    Load(LoadError),
    Json(JsonError),
    Compile(CompileError),
    /// 清单缺少分片。
    EmptyShards,
    /// 装载后无可用 Locale。
    NoAvailableLocales,
}

impl LocaleLoadError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Load(e) => e.code(),
            Self::Json(e) => e.code(),
            Self::Compile(e) => e.code(),
            Self::EmptyShards => "spark.localization.empty_shards",
            Self::NoAvailableLocales => "spark.localization.no_available_locales",
        }
    }
}

impl fmt::Display for LocaleLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for LocaleLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Load(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::Compile(e) => Some(e),
            Self::EmptyShards | Self::NoAvailableLocales => None,
        }
    }
}

impl From<LoadError> for LocaleLoadError {
    fn from(value: LoadError) -> Self {
        Self::Load(value)
    }
}

impl From<JsonError> for LocaleLoadError {
    fn from(value: JsonError) -> Self {
        Self::Json(value)
    }
}

impl From<CompileError> for LocaleLoadError {
    fn from(value: CompileError) -> Self {
        Self::Compile(value)
    }
}

/// 用 [`AssetLoader`] 读取逻辑路径上的 JSON 语言包。
pub fn load_document_json(
    loader: &dyn AssetLoader,
    key: impl Into<AssetKey>,
) -> Result<LocalizationDocument, LocaleLoadError> {
    let key = key.into();
    let bytes = loader.load(&key)?;
    Ok(document_from_json_slice(&bytes)?)
}

/// 按清单分片键装载并编译完整 [`LocalizationBundle`]。
pub fn load_bundle_from_manifest(
    loader: &dyn AssetLoader,
    manifest: &LocalizationManifest,
) -> Result<LocalizationBundle, LocaleLoadError> {
    let mut documents = Vec::new();
    for paths in manifest.shards.values() {
        for path in paths {
            documents.push(load_document_json(loader, path.as_ref())?);
        }
    }
    if documents.is_empty() {
        return Err(LocaleLoadError::EmptyShards);
    }
    Ok(compile_documents(&documents, CompileOptions::default())?.bundle)
}

/// 装载、协商并构造准备提交的快照（失败时由调用方保留旧快照）。
pub fn prepare_snapshot(
    loader: &dyn AssetLoader,
    manifest: &LocalizationManifest,
    request: &LocaleRequest,
    generation: u64,
) -> Result<LocaleSnapshot, LocaleLoadError> {
    let bundle = load_bundle_from_manifest(loader, manifest)?;
    let available = if manifest.locales.is_empty() {
        bundle.locales()
    } else {
        manifest.available_locales()
    };
    if available.is_empty() {
        return Err(LocaleLoadError::NoAvailableLocales);
    }
    let resolved = negotiate(request, &available, &manifest.product_default);
    Ok(LocaleSnapshot::from_bundle(
        resolved,
        &manifest.product_default,
        &available,
        generation,
        bundle,
    ))
}

/// 内存装载器：逻辑键 → JSON 字节（测试与无磁盘场景）。
#[derive(Debug, Default, Clone)]
pub struct MemoryLocaleLoader {
    files: std::collections::BTreeMap<String, Arc<[u8]>>,
}

impl MemoryLocaleLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: impl Into<String>, bytes: impl AsRef<[u8]>) {
        let slice: Arc<[u8]> = Arc::from(bytes.as_ref());
        self.files.insert(key.into(), slice);
    }
}

impl AssetLoader for MemoryLocaleLoader {
    fn load(&self, key: &AssetKey) -> Result<Vec<u8>, LoadError> {
        self.files
            .get(key.as_str())
            .map(|b| b.as_ref().to_vec())
            .ok_or_else(|| LoadError::not_found(key.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale::LocaleId;
    use crate::manifest::{LocaleEntry, LocalizationManifest};
    use crate::message::{MessageArgs, MessageRef};

    #[test]
    fn loads_json_shards_into_snapshot() {
        let mut loader = MemoryLocaleLoader::new();
        loader.insert(
            "locales/en/common.json",
            r#"{
              "locale": "en",
              "namespace": "game",
              "messages": { "menu.quit": "Quit" }
            }"#,
        );
        loader.insert(
            "locales/zh-Hans/common.json",
            r#"{
              "locale": "zh-Hans",
              "namespace": "game",
              "messages": { "menu.quit": "退出" }
            }"#,
        );

        let mut manifest = LocalizationManifest::new(LocaleId::parse("en").unwrap());
        manifest.locales.push(LocaleEntry {
            locale: LocaleId::parse("en").unwrap(),
            fallback: vec![],
            bundled: true,
            font_hint: None,
        });
        manifest.locales.push(LocaleEntry {
            locale: LocaleId::parse("zh-Hans").unwrap(),
            fallback: vec![],
            bundled: true,
            font_hint: None,
        });
        manifest.shards.insert(
            LocaleId::parse("en").unwrap(),
            vec![Arc::from("locales/en/common.json")],
        );
        manifest.shards.insert(
            LocaleId::parse("zh-Hans").unwrap(),
            vec![Arc::from("locales/zh-Hans/common.json")],
        );

        let request = LocaleRequest::new(vec![LocaleId::parse("zh-Hans-CN").unwrap()], vec![]);
        let snap = prepare_snapshot(&loader, &manifest, &request, 7).unwrap();
        assert_eq!(snap.locale.as_str(), "zh-Hans");
        assert_eq!(snap.generation, 7);
        let text = snap.format(&MessageRef::named("game", "menu.quit"), &MessageArgs::new());
        assert_eq!(text.text.as_ref(), "退出");
    }
}
