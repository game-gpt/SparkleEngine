//! Spark **RTS** 特异化引擎壳。
//!
//! 在 [`spark_engine::SparkEngine`] 之上提供即时战略常见运行时骨架：
//! 单位句柄、框选、指令队列、编制、迷雾格子。
//! **禁止**阵营科技树、具体兵种表、地图战役——那些属于游戏仓。

mod command;
mod fog;
mod selection;
mod unit;

pub use command::{Command, CommandQueue};
pub use fog::FogGrid;
pub use selection::Selection;
pub use unit::{PlayerId, UnitId, UnitPose, UnitRoster};

use spark_core::Vec2;
use spark_engine::SparkEngine;
use std::path::PathBuf;

/// RTS 会话：模组引擎 + 单位/指令/视野。
pub struct RtsEngine {
    pub engine: SparkEngine,
    pub roster: UnitRoster,
    pub selection: Selection,
    pub commands: CommandQueue,
    pub fog: FogGrid,
}

impl RtsEngine {
    pub fn new(mods_root: impl Into<PathBuf>, map_w: u32, map_h: u32, cell: f32) -> Self {
        Self {
            engine: SparkEngine::new(mods_root),
            roster: UnitRoster::new(),
            selection: Selection::default(),
            commands: CommandQueue::default(),
            fog: FogGrid::new(map_w, map_h, cell),
        }
    }

    /// 固定步：推进指令并刷新迷雾（由当前玩家单位揭示）。
    pub fn tick(&mut self, dt: f32, local_player: PlayerId) {
        self.commands
            .dispatch(dt, &mut self.roster, local_player);
        self.fog.clear_visible();
        for u in self.roster.iter() {
            if u.owner == local_player && u.alive {
                self.fog.reveal(u.pose.pos, u.sight_radius);
            }
        }
    }

    /// 世界坐标点选最近单位（可选阵营过滤）。
    pub fn pick(&self, world: Vec2, max_dist: f32, owner: Option<PlayerId>) -> Option<UnitId> {
        self.roster.pick_nearest(world, max_dist, owner)
    }

    /// 矩形框选（世界轴对齐）。
    pub fn box_select(&mut self, min: Vec2, max: Vec2, owner: Option<PlayerId>) {
        let ids = self.roster.query_rect(min, max, owner);
        self.selection.set(ids);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_core::Vec2;

    #[test]
    fn select_and_move() {
        let mut rts = RtsEngine::new(".", 32, 32, 1.0);
        let p = PlayerId(1);
        let a = rts.roster.spawn(p, UnitPose::at(2.0, 2.0), 4.0);
        let b = rts.roster.spawn(p, UnitPose::at(10.0, 10.0), 4.0);
        rts.box_select(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0), Some(p));
        assert_eq!(rts.selection.ids(), &[a]);
        rts.commands
            .issue(a, Command::MoveTo { target: Vec2::new(8.0, 2.0), speed: 4.0 });
        rts.tick(1.0, p);
        let u = rts.roster.get(a).unwrap();
        assert!((u.pose.pos.x - 6.0).abs() < 1e-3);
        assert!(rts.fog.is_visible(2, 2));
        let _ = b;
    }
}
