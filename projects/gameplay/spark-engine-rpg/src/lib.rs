//! Spark **RPG** 特异化引擎壳。
//!
//! 队伍、背包、通用属性表、任务簿、简易回合序。
//! **禁止**职业树、技能表、具体道具定义——那些属于游戏仓。

#![warn(missing_docs)]
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

/// RPG 会话。
pub struct RpgEngine {
    pub engine: SparkEngine,
    pub party: Party,
    pub inventory: Inventory,
    pub quests: QuestJournal,
    pub turns: TurnOrder,
}

impl RpgEngine {
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
