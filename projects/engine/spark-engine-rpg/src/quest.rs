//! 任务簿。

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QuestId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestStatus {
    Inactive,
    Active,
    Completed,
    Failed,
}

#[derive(Debug, Default)]
pub struct QuestJournal {
    entries: HashMap<QuestId, QuestStatus>,
}

impl QuestJournal {
    pub fn upsert(&mut self, id: QuestId, status: QuestStatus) {
        self.entries.insert(id, status);
    }

    pub fn status(&self, id: &QuestId) -> Option<QuestStatus> {
        self.entries.get(id).copied()
    }

    pub fn complete(&mut self, id: &QuestId) {
        if let Some(s) = self.entries.get_mut(id) {
            *s = QuestStatus::Completed;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&QuestId, QuestStatus)> {
        self.entries.iter().map(|(k, v)| (k, *v))
    }
}
