//! 单位指令队列（移动 / 攻击目标点 / 停止）。
//!
//! 同一单位新指令会替换旧指令。[`CommandQueue::dispatch`] 到达目标或 `Stop` 后移除条目。

use spark_types::Vec2;

use crate::unit::{PlayerId, UnitId, UnitRoster};

/// 可下发给单位的指令。
#[derive(Debug, Clone)]
pub enum Command {
    /// 移向目标点。
    MoveTo {
        /// 世界坐标目标。
        target: Vec2,
        /// 移动速度（世界单位 / 秒）。
        speed: f32,
    },
    /// 攻击移动（本壳与 MoveTo 同路径推进，战斗语义留给游戏仓）。
    AttackMove {
        /// 世界坐标目标。
        target: Vec2,
        /// 移动速度。
        speed: f32,
    },
    /// 立即停止并清除该单位指令。
    Stop,
}

#[derive(Debug, Clone)]
struct Queued {
    unit: UnitId,
    cmd: Command,
}

/// 每单位至多一条挂起指令的队列。
#[derive(Debug, Default)]
pub struct CommandQueue {
    items: Vec<Queued>,
}

impl CommandQueue {
    /// 下发或替换某单位的指令。
    pub fn issue(&mut self, unit: UnitId, cmd: Command) {
        self.items.retain(|q| q.unit != unit);
        self.items.push(Queued { unit, cmd });
    }

    /// 对多个单位各下发同一指令的克隆。
    pub fn issue_selection(&mut self, units: &[UnitId], cmd: Command) {
        for &u in units {
            self.issue(u, cmd.clone());
        }
    }

    /// 清除某单位挂起指令。
    pub fn clear_unit(&mut self, unit: UnitId) {
        self.items.retain(|q| q.unit != unit);
    }

    /// 推进一帧；`_local` 预留权限校验。
    pub fn dispatch(&mut self, dt: f32, roster: &mut UnitRoster, _local: PlayerId) {
        let mut done = Vec::new();
        for (i, q) in self.items.iter().enumerate() {
            let Some(u) = roster.get_mut(q.unit)
            else {
                done.push(i);
                continue;
            };
            match &q.cmd {
                Command::Stop => {
                    done.push(i);
                }
                Command::MoveTo { target, speed } | Command::AttackMove { target, speed } => {
                    let dx = target.x - u.pose.pos.x;
                    let dy = target.y - u.pose.pos.y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let step = (*speed).max(0.0) * dt;
                    if dist <= step || dist < 1e-4 {
                        u.pose.pos = *target;
                        done.push(i);
                    }
                    else {
                        u.pose.pos.x += dx / dist * step;
                        u.pose.pos.y += dy / dist * step;
                        u.pose.facing = dy.atan2(dx);
                    }
                }
            }
        }
        for i in done.into_iter().rev() {
            self.items.swap_remove(i);
        }
    }
}
