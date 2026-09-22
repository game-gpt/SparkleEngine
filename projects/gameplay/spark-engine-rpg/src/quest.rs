//! 任务簿。

use std::collections::HashMap;

/// 任务键（游戏仓字符串，不透明）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QuestId(pub String);

/// 任务生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestStatus {
    /// 未接取 / 未激活。
    Inactive,
    /// 进行中。
    Active,
    /// 已完成。
    Completed,
    /// 失败（可再由游戏仓重置）。
    Failed,
}

/// 任务 ID → 状态 映射。
#[derive(Debug, Default)]
pub struct QuestJournal {
    entries: HashMap<QuestId, QuestStatus>,
}

impl QuestJournal {
    /// 插入或覆盖任务状态。
    pub fn upsert(&mut self, id: QuestId, status: QuestStatus) {
        self.entries.insert(id, status);
    }

    /// 查询状态；未知任务返回 `None`。
    pub fn status(&self, id: &QuestId) -> Option<QuestStatus> {
        self.entries.get(id).copied()
    }

    /// 若存在则标为 [`QuestStatus::Completed`]；不存在则无操作。
    pub fn complete(&mut self, id: &QuestId) {
        if let Some(s) = self.entries.get_mut(id) {
            *s = QuestStatus::Completed;
        }
    }

    /// 遍历全部条目。
    pub fn iter(&self) -> impl Iterator<Item = (&QuestId, QuestStatus)> {
        self.entries.iter().map(|(k, v)| (k, *v))
    }
}
