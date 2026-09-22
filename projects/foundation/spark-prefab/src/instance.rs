//! Prefab 实例：引用 + 字段覆盖补丁。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use spark_asset::{AssetRef, MetaValue};

use crate::{document::PrefabDocument, error::PrefabError};

/// 场景或父 Prefab 中的实例记录。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefabInstance {
    /// 固定判别：`prefab-instance`。
    #[serde(rename = "type")]
    pub kind: String,
    /// 指向 Prefab 资源（路径为主）。
    pub prefab: AssetRef,
    /// 实例本地稳定 ID（Agent 可读，非 UUID）。
    #[serde(rename = "instanceId")]
    pub instance_id: String,
    /// 覆盖：键为 [`crate::r#override::OverridePath`] 源格式。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub overrides: BTreeMap<String, MetaValue>,
}

impl PrefabInstance {
    /// 新建空覆盖实例。
    pub fn new(prefab: impl Into<AssetRef>, instance_id: impl Into<String>) -> Self {
        Self { kind: "prefab-instance".into(), prefab: prefab.into(), instance_id: instance_id.into(), overrides: BTreeMap::new() }
    }

    /// 设置或替换一条覆盖（幂等）。
    pub fn set_override(&mut self, path: impl Into<String>, value: MetaValue) {
        self.overrides.insert(path.into(), value);
    }

    /// 对照已加载的 Prefab 文档校验全部覆盖键。
    pub fn validate_against(&self, doc: &PrefabDocument) -> Result<(), PrefabError> {
        doc.validate()?;
        for key in self.overrides.keys() {
            doc.validate_override_key(key)?;
        }
        Ok(())
    }
}
