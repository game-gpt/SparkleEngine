//! 产品本地化清单（对应 `localization.von` 的逻辑模型）。
//!
//! 作者清单使用 Oak VON；JSON 仅作交换格式。清单不得写死本机路径；
//! 资产一律经 `spark-asset` 逻辑路径解析。

use std::{collections::BTreeMap, sync::Arc};

use crate::{locale::LocaleId, message::NamespaceId};

/// 单个 Locale 的安装与回退声明。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleEntry {
    /// Locale 标识。
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
    /// 被登记的命名空间。
    pub namespace: NamespaceId,
    /// 所有权声明（模组 / 产品 ID，非展示文案）。
    pub owner: Arc<str>,
    /// 是否允许外部包显式覆写本命名空间键。
    pub allow_override: bool,
}

/// 产品本地化清单。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationManifest {
    /// 产品默认 Locale（协商终点之一）。
    pub product_default: LocaleId,
    /// 已声明 Locale 列表。
    pub locales: Vec<LocaleEntry>,
    /// 命名空间所有权表。
    pub namespaces: Vec<NamespaceOwner>,
    /// CLDR / 文化数据版本标签（供裁剪与诊断）。
    pub cldr_version: Arc<str>,
    /// 编译器 / 包格式版本。
    pub format_version: u32,
    /// 资产分片：locale → 逻辑资源路径列表。
    pub shards: BTreeMap<LocaleId, Vec<Arc<str>>>,
}

impl LocalizationManifest {
    /// 以产品默认 Locale 构造空清单（`cldr_version` 为 `unspecified`，格式版本 1）。
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

    /// 清单中声明的全部 Locale。
    pub fn available_locales(&self) -> Vec<LocaleId> {
        self.locales.iter().map(|entry| entry.locale.clone()).collect()
    }

    /// 标记为 `bundled` 的 Locale 子集。
    pub fn bundled_locales(&self) -> Vec<LocaleId> {
        self.locales.iter().filter(|entry| entry.bundled).map(|entry| entry.locale.clone()).collect()
    }
}
