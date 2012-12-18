//! 产品本地化清单（对应 `localization.von` 的逻辑模型）。
//!
//! 作者清单使用 Oak VON；JSON 仅作交换格式。清单不得写死本机路径；
//! 资产一律经 `spark-asset` 逻辑路径解析。

use std::{collections::BTreeMap, sync::Arc};

use crate::{locale::LocaleId, message::NamespaceId};

/// 单个 Locale 的安装与回退声明。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleEntry {
    pub locale: LocaleId,
    /// 相对产品默认的额外 fallback（先于父 Locale 展开）。
    pub fallback: Vec<LocaleId>,
    /// 是否随首包安装。
    pub bundled: bool,
    /// 可选字体提示（逻辑资源名，非本机路径）。
    pub font_hint: Option<Arc<str>>,
}

/// 命名空间所有者登记。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceOwner {
    pub namespace: NamespaceId,
    pub owner: Arc<str>,
    /// 是否允许外部包显式覆写本命名空间键。
    pub allow_override: bool,
}

/// 产品本地化清单。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationManifest {
    pub product_default: LocaleId,
    pub locales: Vec<LocaleEntry>,
    pub namespaces: Vec<NamespaceOwner>,
    /// CLDR / 文化数据版本标签（供裁剪与诊断）。
    pub cldr_version: Arc<str>,
    /// 编译器 / 包格式版本。
    pub format_version: u32,
    /// 资产分片：locale → 逻辑资源路径列表。
    pub shards: BTreeMap<LocaleId, Vec<Arc<str>>>,
}

impl LocalizationManifest {
    pub fn new(product_default: LocaleId) -> Self {
        Self {
            product_default,
            locales: Vec::new(),
            namespaces: Vec::new(),
            cldr_version: Arc::from("unspecified"),
            format_version: 1,
            shards: BTreeMap::new(),
        }
    }

    pub fn available_locales(&self) -> Vec<LocaleId> {
        self.locales.iter().map(|entry| entry.locale.clone()).collect()
    }

    pub fn bundled_locales(&self) -> Vec<LocaleId> {
        self.locales.iter().filter(|entry| entry.bundled).map(|entry| entry.locale.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_filter() {
        let mut manifest = LocalizationManifest::new(LocaleId::parse("en").unwrap());
        manifest.locales.push(LocaleEntry { locale: LocaleId::parse("en").unwrap(), fallback: vec![], bundled: true, font_hint: None });
        manifest.locales.push(LocaleEntry {
            locale: LocaleId::parse("ja-JP").unwrap(),
            fallback: vec![],
            bundled: false,
            font_hint: Some(Arc::from("fonts/cjk")),
        });
        assert_eq!(manifest.bundled_locales().len(), 1);
        assert_eq!(manifest.available_locales().len(), 2);
    }
}
