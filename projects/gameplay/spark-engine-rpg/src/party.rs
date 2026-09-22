//! 队伍成员。

use crate::stats::StatSheet;

/// 队伍内角色稳定 ID（会话内单调递增）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActorId(pub u32);

/// 一名队员：键 + 属性表 + 是否参战。
#[derive(Debug, Clone)]
pub struct PartyMember {
    /// 成员 ID。
    pub id: ActorId,
    /// 游戏侧角色键（非显示名权威）。
    pub key: String,
    /// 通用属性表。
    pub stats: StatSheet,
    /// 是否出现在当前战斗/回合池（默认 `true`）。
    pub in_battle: bool,
}

/// 可变队伍列表。
#[derive(Debug, Default)]
pub struct Party {
    next: u32,
    members: Vec<PartyMember>,
}

impl Party {
    /// 加入新成员，返回新 [`ActorId`]；属性表为空、`in_battle = true`。
    pub fn add(&mut self, key: impl Into<String>) -> ActorId {
        let id = ActorId(self.next);
        self.next = self.next.saturating_add(1);
        self.members.push(PartyMember { id, key: key.into(), stats: StatSheet::default(), in_battle: true });
        id
    }

    /// 按 ID 只读查找。
    pub fn get(&self, id: ActorId) -> Option<&PartyMember> {
        self.members.iter().find(|m| m.id == id)
    }

    /// 按 ID 可变查找。
    pub fn get_mut(&mut self, id: ActorId) -> Option<&mut PartyMember> {
        self.members.iter_mut().find(|m| m.id == id)
    }

    /// 遍历全部成员。
    pub fn iter(&self) -> impl Iterator<Item = &PartyMember> {
        self.members.iter()
    }

    /// 移除成员；不存在则无操作。不回收 ID。
    pub fn remove(&mut self, id: ActorId) {
        self.members.retain(|m| m.id != id);
    }

    /// 当前人数。
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// 是否无人。
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}
