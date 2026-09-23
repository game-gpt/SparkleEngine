//! 运行期列批量视图：在 [`QueryPlan`] 与 [`ScriptQuerySnapshot`] 之上提供无字符串热路径访问。

use crate::query_plan::QueryPlan;
use crate::query_view::ScriptQuerySnapshot;

/// 单一原型的实体列视图（连续 `entity_bits` 切片）。
#[derive(Debug, Clone)]
pub struct ScriptColumnView {
    archetype_index: u32,
    entities: Vec<u64>,
}

impl ScriptColumnView {
    /// 本列实体行数。
    pub fn count(&self) -> usize {
        self.entities.len()
    }

    /// 绑定期原型下标（与 [`QueryPlan::archetype_name`] 对齐）。
    pub fn archetype_index(&self) -> u32 {
        self.archetype_index
    }

    /// 按下标取实体位模式；越界为 `None`。
    pub fn entity_bits(&self, row: usize) -> Option<u64> {
        self.entities.get(row).copied()
    }

    /// 实体位列切片（脚本热循环遍历用）。
    pub fn entities(&self) -> &[u64] {
        &self.entities
    }
}

/// 一次脚本 System 调用的列批量上下文（计划 + 快照）。
#[derive(Debug, Clone)]
pub struct ScriptColumnBatch {
    plan: QueryPlan,
    snapshot: ScriptQuerySnapshot,
    views: Vec<ScriptColumnView>,
}

impl ScriptColumnBatch {
    /// 按绑定期计划与帧快照安装列视图。
    pub fn install(plan: QueryPlan, snapshot: ScriptQuerySnapshot) -> Self {
        let views = if plan.is_unfiltered() {
            snapshot
                .archetype_names()
                .into_iter()
                .enumerate()
                .map(|(i, name)| ScriptColumnView {
                    archetype_index: i as u32,
                    entities: snapshot.entities(name).to_vec(),
                })
                .collect()
        }
        else {
            plan.archetypes()
                .iter()
                .enumerate()
                .map(|(i, name)| ScriptColumnView {
                    archetype_index: i as u32,
                    entities: snapshot.entities(name.as_ref()).to_vec(),
                })
                .collect()
        };
        Self { plan, snapshot, views }
    }

    /// 绑定期查询计划。
    pub fn plan(&self) -> &QueryPlan {
        &self.plan
    }

    /// 底层只读快照。
    pub fn snapshot(&self) -> &ScriptQuerySnapshot {
        &self.snapshot
    }

    /// 已安装的列视图（每原型一列实体 id）。
    pub fn views(&self) -> &[ScriptColumnView] {
        &self.views
    }

    /// 按绑定期原型下标取实体行数（热路径无字符串查找）。
    pub fn entity_count(&self, archetype_index: usize) -> usize {
        self.views.get(archetype_index).map(|v| v.count()).unwrap_or(0)
    }

    /// 总实体行数（全部列之和）。
    pub fn total_entity_count(&self) -> usize {
        self.views.iter().map(|v| v.count()).sum()
    }
}
