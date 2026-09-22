//! 单位指令队列（移动 / 攻击目标点 / 停止）。

use spark_types::Vec2;

use crate::unit::{PlayerId, UnitId, UnitRoster};

#[derive(Debug, Clone)]
pub enum Command {
    MoveTo { target: Vec2, speed: f32 },
    AttackMove { target: Vec2, speed: f32 },
    Stop,
}

#[derive(Debug, Clone)]
struct Queued {
    unit: UnitId,
    cmd: Command,
}

#[derive(Debug, Default)]
pub struct CommandQueue {
    items: Vec<Queued>,
}

impl CommandQueue {
    pub fn issue(&mut self, unit: UnitId, cmd: Command) {
        self.items.retain(|q| q.unit != unit);
        self.items.push(Queued { unit, cmd });
    }

    pub fn issue_selection(&mut self, units: &[UnitId], cmd: Command) {
        for &u in units {
            self.issue(u, cmd.clone());
        }
    }

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
