//! Prefab 声明式源文档。

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use spark_asset::AssetRef;

use crate::error::PrefabError;

/// Prefab JSON `schema` 字面量。
pub const PREFAB_SCHEMA: &str = "spark.prefab";

/// 当前源格式版本。
pub const PREFAB_VERSION: u32 = 1;

/// Prefab 资源本体（Agent / 人类编辑的源文件）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefabDocument {
    /// 固定为 [`PREFAB_SCHEMA`]。
    pub schema: String,
    /// 格式版本。
    pub version: u32,
    /// 根节点本地 ID。
    pub root: String,
    /// 节点表：键为本地 `nodeId`（稳定、可读，不是 UUID）。
    pub nodes: BTreeMap<String, PrefabNode>,
}

/// Prefab 内部节点。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefabNode {
    /// 显示名（可与 `nodeId` 不同）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 组件：类型名 → 字段对象（声明式 JSON，非 ECS 运行时布局）。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub components: BTreeMap<String, serde_json::Value>,
    /// 子节点本地 ID 列表（有序）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<String>,
    /// 嵌套实例：引用另一 Prefab 资源（路径为主）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefab: Option<AssetRef>,
}

impl PrefabDocument {
    /// 空壳：给定根 ID，仅含根节点。
    pub fn new(root_id: impl Into<String>) -> Self {
        let root = root_id.into();
        let mut nodes = BTreeMap::new();
        nodes.insert(
            root.clone(),
            PrefabNode {
                name: Some(root.clone()),
                components: BTreeMap::new(),
                children: Vec::new(),
                prefab: None,
            },
        );
        Self {
            schema: PREFAB_SCHEMA.into(),
            version: PREFAB_VERSION,
            root,
            nodes,
        }
    }

    /// 确保节点存在；已存在则返回可变借用。
    pub fn ensure_node(&mut self, id: impl Into<String>) -> &mut PrefabNode {
        let id = id.into();
        self.nodes.entry(id.clone()).or_insert_with(|| PrefabNode {
            name: Some(id),
            components: BTreeMap::new(),
            children: Vec::new(),
            prefab: None,
        })
    }

    /// 将 `child` 挂到 `parent` 的 children（幂等）。
    pub fn ensure_child(&mut self, parent: &str, child: &str) -> Result<(), PrefabError> {
        if !self.nodes.contains_key(parent) {
            return Err(PrefabError::NodeMissing {
                node: parent.into(),
            });
        }
        self.ensure_node(child);
        let kids = &mut self.nodes.get_mut(parent).expect("parent just checked").children;
        if !kids.iter().any(|c| c == child) {
            kids.push(child.into());
        }
        Ok(())
    }

    /// 确保组件槽存在（空对象）；已有则不动。
    pub fn ensure_component(&mut self, node: &str, component: &str) -> Result<&mut serde_json::Value, PrefabError> {
        if !self.nodes.contains_key(node) {
            return Err(PrefabError::NodeMissing { node: node.into() });
        }
        let entry = self
            .nodes
            .get_mut(node)
            .expect("node just checked")
            .components
            .entry(component.into())
            .or_insert_with(|| serde_json::json!({}));
        Ok(entry)
    }

    /// 设置组件整块值。
    pub fn set_component(&mut self, node: &str, component: &str, value: serde_json::Value) -> Result<(), PrefabError> {
        if !self.nodes.contains_key(node) {
            return Err(PrefabError::NodeMissing { node: node.into() });
        }
        self.nodes
            .get_mut(node)
            .expect("node just checked")
            .components
            .insert(component.into(), value);
        Ok(())
    }

    /// 从 JSON 文本解析。
    pub fn from_str(text: &str) -> Result<Self, PrefabError> {
        serde_json::from_str(text).map_err(|e| PrefabError::Parse {
            path: PathBuf::from("<memory>"),
            detail: e.to_string(),
        })
    }

    /// 序列化为格式化 JSON（末尾换行，便于 Git）。
    pub fn to_string_pretty(&self) -> Result<String, PrefabError> {
        let text = serde_json::to_string_pretty(self).map_err(|e| PrefabError::Parse {
            path: PathBuf::from("<memory>"),
            detail: e.to_string(),
        })?;
        Ok(format!("{text}\n"))
    }

    /// 读盘。
    pub fn load(path: impl AsRef<Path>) -> Result<Self, PrefabError> {
        let path = path.as_ref();
        let text = fs::read_to_string(path).map_err(|e| PrefabError::Io {
            path: path.to_path_buf(),
            detail: e.to_string(),
        })?;
        serde_json::from_str(&text).map_err(|e| PrefabError::Parse {
            path: path.to_path_buf(),
            detail: e.to_string(),
        })
    }

    /// 写盘。
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), PrefabError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| PrefabError::Io {
                    path: parent.to_path_buf(),
                    detail: e.to_string(),
                })?;
            }
        }
        let text = self.to_string_pretty()?;
        fs::write(path, text).map_err(|e| PrefabError::Io {
            path: path.to_path_buf(),
            detail: e.to_string(),
        })
    }
}
