//! 队伍成员。

use crate::stats::StatSheet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActorId(pub u32);

#[derive(Debug, Clone)]
pub struct PartyMember {
    pub id: ActorId,
    /// 游戏侧角色键（非显示名权威）。
    pub key: String,
    pub stats: StatSheet,
    pub in_battle: bool,
}

#[derive(Debug, Default)]
pub struct Party {
    next: u32,
    members: Vec<PartyMember>,
}

impl Party {
    pub fn add(&mut self, key: impl Into<String>) -> ActorId {
        let id = ActorId(self.next);
        self.next = self.next.saturating_add(1);
        self.members.push(PartyMember { id, key: key.into(), stats: StatSheet::default(), in_battle: true });
        id
    }

    pub fn get(&self, id: ActorId) -> Option<&PartyMember> {
        self.members.iter().find(|m| m.id == id)
    }

    pub fn get_mut(&mut self, id: ActorId) -> Option<&mut PartyMember> {
        self.members.iter_mut().find(|m| m.id == id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &PartyMember> {
        self.members.iter()
    }

    pub fn remove(&mut self, id: ActorId) {
        self.members.retain(|m| m.id != id);
    }

    pub fn len(&self) -> usize {
        self.members.len()
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}
