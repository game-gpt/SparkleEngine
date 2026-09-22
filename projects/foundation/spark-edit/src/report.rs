//! 执行报告与变更计划。

use serde::Serialize;

use crate::diagnostic::Diagnostic;

/// 事务结果状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionState {
    /// 仅校验。
    Checked,
    /// 已生成变更计划，未落盘。
    Planned,
    /// 已写入。
    Applied,
    /// 已回滚（失败时未落盘或部分撤销）。
    RolledBack,
}

/// 变更种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// 新建文件。
    Create,
    /// 更新文件。
    Update,
    /// 写入旁车 `.meta`。
    WriteMeta,
}

/// 单条变更。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangeRecord {
    /// 种类。
    pub kind: ChangeKind,
    /// 路径（相对或绝对，与计划一致）。
    pub path: String,
}

/// 一次 shell / MCP 调用的结构化结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EditReport {
    /// 是否成功（无 error 级诊断）。
    pub ok: bool,
    /// 执行模式。
    pub mode: String,
    /// 诊断列表。
    pub diagnostics: Vec<Diagnostic>,
    /// 变更（dry-run 为计划；apply 为已发生）。
    pub changes: Vec<ChangeRecord>,
    /// 事务状态。
    pub transaction: TransactionState,
}

impl EditReport {
    /// 序列化为 JSON（MCP / `--json`）。
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{\"ok\":false}".into())
    }
}
