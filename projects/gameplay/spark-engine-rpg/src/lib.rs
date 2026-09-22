//! Spark **RPG** 特异化引擎壳。
//!
//! 队伍、背包、通用属性表、任务簿、简易回合序。
//! **禁止**职业树、技能表、具体道具定义——那些属于游戏仓。

#![forbid(missing_docs)]
mod inventory;
mod party;
mod quest;
mod stats;
mod turn;

pub use inventory::{Inventory, ItemStack};
pub use party::{ActorId, Party, PartyMember};
pub use quest::{QuestId, QuestJournal, QuestStatus};
pub use stats::StatSheet;
pub use turn::TurnOrder;

use spark_engine::SparkEngine;
use std::path::PathBuf;

/// RPG 会话：在 [`SparkEngine`] 之上挂接队伍 / 背包 / 任务 / 回合序。
///
/// 不拥有职业或道具权威表；`ItemStack::id` / 角色 `key` 由游戏仓解释。
pub struct RpgEngine {
    /// 底层模组 / 资源 / 帧循环宿主。
    pub engine: SparkEngine,
    /// 当前队伍。
    pub party: Party,
    /// 默认 30 格堆叠背包。
    pub inventory: Inventory,
    /// 任务状态簿。
    pub quests: QuestJournal,
    /// 战斗/对话用 FIFO 回合序。
    pub turns: TurnOrder,
}

impl RpgEngine {
    /// 以 `mods_root` 构造空会话（背包 30 槽，其余表为空）。
    pub fn new(mods_root: impl Into<PathBuf>) -> Self {
        Self {
            engine: SparkEngine::new(mods_root),
            party: Party::default(),
            inventory: Inventory::with_slots(30),
            quests: QuestJournal::default(),
            turns: TurnOrder::default(),
        }
    }
}
