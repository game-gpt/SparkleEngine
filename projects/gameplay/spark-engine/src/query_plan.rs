//! 绑定期 [`QueryPlan`]：将脚本 System 描述符解析为稳定槽位，热路径禁止再按名字查找。

use std::sync::Arc;

use crate::command_apply::{ComponentDescriptorId, ScriptComponentCatalog};
use crate::script_system::ScriptSystemDescriptor;

/// 查询计划绑定失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryPlanError {
    /// 组件名未在 [`ScriptComponentCatalog`] 登记。
    UnknownComponent {
        /// 未登记的组件逻辑名。
        name: String,
    },
}

impl std::fmt::Display for QueryPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownComponent { name } => write!(f, "spark.engine.query_plan.unknown_component:{name}"),
        }
    }
}

impl std::error::Error for QueryPlanError {}

/// 绑定期解析后的单列访问（槽位 ID + 读写）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundColumn {
    /// 目录内稳定槽位。
    pub slot: ComponentDescriptorId,
    /// `true` = 写；`false` = 只读。
    pub write: bool,
}

/// 脚本 System 的绑定期查询计划（装载 / 登记时生成，运行期只读）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryPlan {
    /// 所属模组 id。
    pub mod_id: Arc<str>,
    /// System 逻辑名。
    pub system_name: Arc<str>,
    /// 原型过滤；`unfiltered` 为 true 时运行期可见快照内全部原型。
    archetypes: Vec<Arc<str>>,
    unfiltered: bool,
    /// 已解析的组件列槽。
    columns: Vec<BoundColumn>,
}

impl QueryPlan {
    /// 从描述符与组件目录绑定；未知组件名立即失败。
    pub fn bind(desc: &ScriptSystemDescriptor, catalog: &ScriptComponentCatalog) -> Result<Self, QueryPlanError> {
        let mut columns = Vec::with_capacity(desc.access.len());
        for access in &desc.access {
            let Some(slot) = catalog.id_of(access.component.as_ref()) else {
                return Err(QueryPlanError::UnknownComponent { name: access.component.to_string() });
            };
            columns.push(BoundColumn { slot, write: access.write });
        }
        let unfiltered = desc.query_archetypes.is_empty();
        let archetypes = if unfiltered {
            Vec::new()
        }
        else {
            desc.query_archetypes.clone()
        };
        Ok(Self {
            mod_id: Arc::clone(&desc.mod_id),
            system_name: Arc::clone(&desc.name),
            archetypes,
            unfiltered,
            columns,
        })
    }

    /// 调度图键：`mod_id/system_name`。
    pub fn graph_key(&self) -> String {
        format!("{}/{}", self.mod_id, self.system_name)
    }

    /// 是否不按原型过滤（可见快照内全部原型）。
    pub fn is_unfiltered(&self) -> bool {
        self.unfiltered
    }

    /// 绑定期解析的原型名列表；未过滤时为空（运行期遍历快照键）。
    pub fn archetypes(&self) -> &[Arc<str>] {
        &self.archetypes
    }

    /// 绑定期解析的组件列。
    pub fn columns(&self) -> &[BoundColumn] {
        &self.columns
    }

    /// 运行期按绑定期下标取原型名；越界返回 `None`。
    pub fn archetype_name(&self, index: usize) -> Option<&str> {
        if self.unfiltered {
            return None;
        }
        self.archetypes.get(index).map(|s| s.as_ref())
    }

    /// 绑定期原型数量；未过滤时由 [`ScriptColumnBatch::archetype_count`] 使用快照键数。
    pub fn archetype_slot_count(&self) -> usize {
        if self.unfiltered {
            0
        }
        else {
            self.archetypes.len()
        }
    }
}
