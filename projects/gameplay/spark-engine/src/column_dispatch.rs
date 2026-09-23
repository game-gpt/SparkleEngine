//! 绑定期列直写分发表：将 [`QueryPlan`] 列 + 字段下标解析为稳定槽位与偏移。

use crate::command_apply::{ComponentDescriptorId, ScriptComponentCatalog};
use crate::query_plan::QueryPlan;
use crate::script_component_schema::ScriptFieldKind;

/// 绑定期解析的单字段直写描述（热路径无布局表查找）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundFieldDispatch {
    /// 组件目录槽位。
    pub slot: ComponentDescriptorId,
    /// 字段字节偏移。
    pub offset: u32,
    /// 物理类型。
    pub kind: ScriptFieldKind,
    /// 所属列是否声明写（来自 [`crate::query_plan::BoundColumn`]）。
    pub column_write: bool,
}

/// 一次脚本 System 调用的列字段分发表（与 [`QueryPlan`] 列下标对齐）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColumnDispatchTable {
    columns: Vec<Vec<BoundFieldDispatch>>,
}

impl ColumnDispatchTable {
    /// 从已绑定 [`QueryPlan`] 与组件目录生成字段分发表。
    pub fn bind(plan: &QueryPlan, catalog: &ScriptComponentCatalog) -> Self {
        let columns = plan
            .columns()
            .iter()
            .map(|col| {
                catalog
                    .layout_of(col.slot)
                    .map(|layout| {
                        layout
                            .fields
                            .iter()
                            .map(|field| BoundFieldDispatch {
                                slot: col.slot,
                                offset: field.offset,
                                kind: field.kind,
                                column_write: col.write,
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .collect();
        Self { columns }
    }

    /// 按列下标 + 字段下标取绑定期条目。
    pub fn field(&self, column_index: usize, field_index: usize) -> Option<&BoundFieldDispatch> {
        self.columns.get(column_index).and_then(|fields| fields.get(field_index))
    }

    /// 已绑定列数。
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }
}
