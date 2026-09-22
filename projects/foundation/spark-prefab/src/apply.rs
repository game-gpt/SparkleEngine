//! 将实例覆盖应用到 Prefab 文档副本（预览 / 展开用，不回写源 Prefab）。

use std::collections::BTreeMap;

use spark_asset::MetaValue;

use crate::{document::PrefabDocument, error::PrefabError, instance::PrefabInstance, r#override::parse_override_path};

impl PrefabDocument {
    /// 在副本上应用实例覆盖，返回新文档。
    ///
    /// 只改字段值；不修改节点图、不写盘。覆盖键须先通过 [`Self::validate_override_key`]。
    pub fn with_overrides(&self, instance: &PrefabInstance) -> Result<Self, PrefabError> {
        instance.validate_against(self)?;
        let mut out = self.clone();
        for (key, value) in &instance.overrides {
            apply_one(&mut out, key, value.clone())?;
        }
        Ok(out)
    }
}

fn apply_one(doc: &mut PrefabDocument, key: &str, value: MetaValue) -> Result<(), PrefabError> {
    let parsed = parse_override_path(key)?;
    let node_id = parsed.node_path.rsplit('/').next().unwrap_or(parsed.node_path.as_str()).to_string();
    let node = doc.nodes.get_mut(&node_id).ok_or_else(|| PrefabError::OverrideTargetMissing { target: key.into() })?;
    let component = node.components.get_mut(&parsed.component).ok_or_else(|| PrefabError::OverrideTargetMissing { target: key.into() })?;
    match component {
        MetaValue::Table(map) => {
            map.insert(parsed.field, value);
        }
        other => {
            let mut map = BTreeMap::new();
            map.insert(parsed.field, value);
            *other = MetaValue::Table(map);
        }
    }
    Ok(())
}
