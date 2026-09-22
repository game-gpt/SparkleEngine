//! 实例覆盖路径：`<nodePath>/<Component>.<field>`。

use crate::error::PrefabError;

/// 解析后的覆盖地址。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverridePath {
    /// 节点路径，如 `player/body`。
    pub node_path: String,
    /// 组件类型名，如 `Transform`。
    pub component: String,
    /// 字段名，如 `position`。
    pub field: String,
}

/// 解析 `player/body/Transform.position`。
pub fn parse_override_path(raw: &str) -> Result<OverridePath, PrefabError> {
    let Some((left, field)) = raw.rsplit_once('.')
    else {
        return Err(PrefabError::BadOverridePath { target: raw.into() });
    };
    let Some((node_path, component)) = left.rsplit_once('/')
    else {
        return Err(PrefabError::BadOverridePath { target: raw.into() });
    };
    if node_path.is_empty() || component.is_empty() || field.is_empty() {
        return Err(PrefabError::BadOverridePath { target: raw.into() });
    }
    if component.contains('.') || field.contains('/') {
        return Err(PrefabError::BadOverridePath { target: raw.into() });
    }
    Ok(OverridePath { node_path: node_path.into(), component: component.into(), field: field.into() })
}

impl OverridePath {
    /// 格式化回源键。
    pub fn to_key(&self) -> String {
        format!("{}/{}.{}", self.node_path, self.component, self.field)
    }
}
