//! Sparkle Script Edit profile 宿主面：具名调用 → [`EditOp`]。
//!
//! 完整脚本 VM 接通前，Agent / 工具可发 `calls` 列表，由本模块降级为编辑计划。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use spark_asset::MetaValue;

use crate::op::EditOp;
use crate::plan::EditPlan;

/// Edit profile 标识（与 `spark-script` 的 profile 名对齐）。
pub const EDIT_PROFILE_ID: &str = "spark-edit-1";

/// 宿主调用名（产品面；实现层仍走 [`EditOp`]）。
pub mod names {
    /// `meta.create`
    pub const META_CREATE: &str = "assets.meta_create";
    /// `meta.load`
    pub const META_LOAD: &str = "assets.meta_load";
    /// `asset.rename`
    pub const ASSET_RENAME: &str = "assets.rename";
    /// `index.scan`
    pub const INDEX_SCAN: &str = "assets.index_scan";
    /// `prefab.ensure`
    pub const PREFAB_ENSURE: &str = "prefabs.ensure";
    /// `prefab.ensure_node`
    pub const PREFAB_ENSURE_NODE: &str = "prefabs.ensure_node";
    /// `prefab.ensure_component`
    pub const PREFAB_ENSURE_COMPONENT: &str = "prefabs.ensure_component";
    /// `prefab.set_field`
    pub const PREFAB_SET_FIELD: &str = "prefabs.set_field";
    /// `prefab.validate`
    pub const PREFAB_VALIDATE: &str = "prefabs.validate";
    /// `prefab.save`
    pub const PREFAB_SAVE: &str = "prefabs.save";
}

/// 单次宿主调用（脚本友好）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostCall {
    /// 调用名，见 [`names`]。
    pub name: String,
    /// 参数表。
    #[serde(default)]
    pub args: BTreeMap<String, MetaValue>,
}

/// 带 profile 的调用脚本（可降级为 [`EditPlan`]）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditHostScript {
    /// 应为 [`EDIT_PROFILE_ID`]。
    #[serde(default)]
    pub profile: Option<String>,
    /// 可选事务名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction: Option<String>,
    /// 调用序列。
    #[serde(default)]
    pub calls: Vec<HostCall>,
}

impl EditHostScript {
    /// 从 VON 解析。
    pub fn from_von(text: &str) -> Result<Self, String> {
        oak_von::from_str(text).map_err(|e| e.to_string())
    }

    /// 降级为编辑计划。
    pub fn lower(&self) -> Result<EditPlan, String> {
        if let Some(profile) = &self.profile {
            if profile != EDIT_PROFILE_ID {
                return Err(format!("unsupported edit profile: {profile}"));
            }
        }
        let mut ops = Vec::with_capacity(self.calls.len());
        for call in &self.calls {
            ops.push(lower_host_call(&call.name, &call.args)?);
        }
        Ok(EditPlan {
            name: self.transaction.clone(),
            ops,
        })
    }
}

fn arg_string(args: &BTreeMap<String, MetaValue>, key: &str) -> Result<String, String> {
    match args.get(key) {
        Some(MetaValue::String(s)) => Ok(s.clone()),
        Some(other) => Err(format!("arg `{key}` must be string, got {other:?}")),
        None => Err(format!("missing arg `{key}`")),
    }
}

fn arg_string_opt(args: &BTreeMap<String, MetaValue>, key: &str) -> Result<Option<String>, String> {
    match args.get(key) {
        None => Ok(None),
        Some(MetaValue::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(format!("arg `{key}` must be string, got {other:?}")),
    }
}

/// 将具名宿主调用降为 [`EditOp`]。
pub fn lower_host_call(name: &str, args: &BTreeMap<String, MetaValue>) -> Result<EditOp, String> {
    match name {
        names::META_CREATE => Ok(EditOp::MetaCreate {
            path: arg_string(args, "path")?,
        }),
        names::META_LOAD => Ok(EditOp::MetaLoad {
            path: arg_string(args, "path")?,
        }),
        names::ASSET_RENAME => Ok(EditOp::AssetRename {
            from: arg_string(args, "from")?,
            to: arg_string(args, "to")?,
        }),
        names::INDEX_SCAN => Ok(EditOp::IndexScan),
        names::PREFAB_ENSURE => Ok(EditOp::PrefabEnsure {
            path: arg_string(args, "path")?,
            root: arg_string(args, "root")?,
        }),
        names::PREFAB_ENSURE_NODE => Ok(EditOp::PrefabEnsureNode {
            path: arg_string(args, "path")?,
            id: arg_string(args, "id")?,
            parent: arg_string_opt(args, "parent")?,
        }),
        names::PREFAB_ENSURE_COMPONENT => Ok(EditOp::PrefabEnsureComponent {
            path: arg_string(args, "path")?,
            node: arg_string(args, "node")?,
            component: arg_string(args, "component")?,
        }),
        names::PREFAB_SET_FIELD => {
            let value = args
                .get("value")
                .cloned()
                .ok_or_else(|| "missing arg `value`".to_string())?;
            Ok(EditOp::PrefabSetField {
                path: arg_string(args, "path")?,
                node: arg_string(args, "node")?,
                component: arg_string(args, "component")?,
                field: arg_string(args, "field")?,
                value,
            })
        }
        names::PREFAB_VALIDATE => Ok(EditOp::PrefabValidate {
            path: arg_string(args, "path")?,
        }),
        names::PREFAB_SAVE => Ok(EditOp::PrefabSave {
            path: arg_string(args, "path")?,
        }),
        other => Err(format!("unknown edit host call: {other}")),
    }
}
