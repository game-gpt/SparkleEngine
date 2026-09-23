//! 运行期列批量视图：在 [`QueryPlan`] 与 [`ScriptQuerySnapshot`] 之上提供无字符串热路径访问。

use std::sync::Arc;

use crate::query_plan::QueryPlan;
use crate::query_view::ScriptQuerySnapshot;

/// 单一原型的实体列绑定（实体 id 从快照切片读取，不在此复制）。
#[derive(Debug, Clone)]
pub struct ScriptColumnView {
    archetype_index: u32,
    archetype: Arc<str>,
}

impl ScriptColumnView {
    /// 本列实体行数。
    pub fn count(&self, snapshot: &ScriptQuerySnapshot) -> usize {
        snapshot.count(self.archetype.as_ref())
    }

    /// 绑定期原型下标（与 [`QueryPlan::archetype_name`] 对齐）。
    pub fn archetype_index(&self) -> u32 {
        self.archetype_index
    }

    /// 按下标取实体位模式；越界为 `None`。
    pub fn entity_bits(&self, snapshot: &ScriptQuerySnapshot, row: usize) -> Option<u64> {
        snapshot.entity_at(self.archetype.as_ref(), row)
    }

    /// 实体位列切片（脚本热循环遍历用）。
    pub fn entities<'a>(&self, snapshot: &'a ScriptQuerySnapshot) -> &'a [u64] {
        snapshot.entities(self.archetype.as_ref())
    }
}

/// 一次脚本 System 调用的列批量上下文（绑定期计划 + 原型槽位）。
#[derive(Debug, Clone)]
pub struct ScriptColumnBatch {
    plan: QueryPlan,
    views: Vec<ScriptColumnView>,
}

impl ScriptColumnBatch {
    /// 按绑定期计划与当前可见快照安装列绑定（不复制实体 id）。
    pub fn install(plan: QueryPlan, snapshot: &ScriptQuerySnapshot) -> Self {
        let views = if plan.is_unfiltered() {
            snapshot
                .archetype_names()
                .into_iter()
                .enumerate()
                .map(|(i, name)| ScriptColumnView { archetype_index: i as u32, archetype: Arc::from(name) })
                .collect()
        }
        else {
            plan.archetypes()
                .iter()
                .enumerate()
                .map(|(i, name)| ScriptColumnView { archetype_index: i as u32, archetype: Arc::clone(name) })
                .collect()
        };
        Self { plan, views }
    }

    /// 绑定期查询计划。
    pub fn plan(&self) -> &QueryPlan {
        &self.plan
    }

    /// 已安装的列绑定（每原型一槽；实体数据从快照读取）。
    pub fn views(&self) -> &[ScriptColumnView] {
        &self.views
    }

    /// 按绑定期原型下标取实体行数（热路径无字符串查找）。
    pub fn entity_count(&self, snapshot: &ScriptQuerySnapshot, archetype_index: usize) -> usize {
        self.views.get(archetype_index).map(|v| v.count(snapshot)).unwrap_or(0)
    }

    /// 按下标取实体位模式；越界为 `None`。
    pub fn entity_bits(&self, snapshot: &ScriptQuerySnapshot, archetype_index: usize, row: usize) -> Option<u64> {
        self.views.get(archetype_index).and_then(|view| view.entity_bits(snapshot, row))
    }

    /// 总实体行数（全部列之和）。
    pub fn total_entity_count(&self, snapshot: &ScriptQuerySnapshot) -> usize {
        self.views.iter().map(|v| v.count(snapshot)).sum()
    }
}
