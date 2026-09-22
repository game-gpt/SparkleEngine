//! 编辑操作（VON / serde）。

use serde::{Deserialize, Serialize};
use spark_asset::MetaValue;

/// 单步编辑操作。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum EditOp {
    /// 为资源创建旁车身份（已存在则失败）。
    #[serde(rename = "meta.create")]
    MetaCreate {
        /// 资源路径。
        path: String,
    },
    /// 读取旁车（缺失则报错）。
    #[serde(rename = "meta.load")]
    MetaLoad {
        /// 资源路径。
        path: String,
    },
    /// 确保 Prefab 文档存在（内存）；`root` 为根节点 ID。
    #[serde(rename = "prefab.ensure")]
    PrefabEnsure {
        /// Prefab 路径。
        path: String,
        /// 根节点 ID。
        root: String,
    },
    /// 确保节点，并可选挂到 `parent`。
    #[serde(rename = "prefab.ensure_node")]
    PrefabEnsureNode {
        /// Prefab 路径。
        path: String,
        /// 节点 ID。
        id: String,
        /// 父节点（可选）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
    },
    /// 确保组件槽（空表）。
    #[serde(rename = "prefab.ensure_component")]
    PrefabEnsureComponent {
        /// Prefab 路径。
        path: String,
        /// 节点 ID。
        node: String,
        /// 组件类型名。
        component: String,
    },
    /// 设置组件字段。
    #[serde(rename = "prefab.set_field")]
    PrefabSetField {
        /// Prefab 路径。
        path: String,
        /// 节点 ID。
        node: String,
        /// 组件类型名。
        component: String,
        /// 字段名。
        field: String,
        /// 字段值。
        value: MetaValue,
    },
    /// 校验并 `save_registered`（写 Prefab + `.meta`）。
    #[serde(rename = "prefab.save")]
    PrefabSave {
        /// Prefab 路径。
        path: String,
    },
}
