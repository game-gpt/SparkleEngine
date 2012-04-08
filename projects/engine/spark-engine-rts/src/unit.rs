//! 单位编制（无兵种数据表）。

use spark_core::Vec2;

/// 玩家 / 控制方不透明 ID（游戏映射阵营）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(pub u32);

/// 单位句柄。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitId(pub u32);

#[derive(Debug, Clone, Copy)]
pub struct UnitPose {
    pub pos: Vec2,
    pub facing: f32,
}

impl UnitPose {
    pub fn at(x: f32, y: f32) -> Self {
        Self {
            pos: Vec2::new(x, y),
            facing: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Unit {
    pub id: UnitId,
    pub owner: PlayerId,
    pub pose: UnitPose,
    pub sight_radius: f32,
    pub alive: bool,
}

#[derive(Debug, Default)]
pub struct UnitRoster {
    next: u32,
    units: Vec<Unit>,
}

impl UnitRoster {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&mut self, owner: PlayerId, pose: UnitPose, sight_radius: f32) -> UnitId {
        let id = UnitId(self.next);
        self.next = self.next.saturating_add(1);
        self.units.push(Unit {
            id,
            owner,
            pose,
            sight_radius,
            alive: true,
        });
        id
    }

    pub fn get(&self, id: UnitId) -> Option<&Unit> {
        self.units.iter().find(|u| u.id == id && u.alive)
    }

    pub fn get_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.units.iter_mut().find(|u| u.id == id && u.alive)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Unit> {
        self.units.iter().filter(|u| u.alive)
    }

    pub fn despawn(&mut self, id: UnitId) {
        if let Some(u) = self.units.iter_mut().find(|u| u.id == id) {
            u.alive = false;
        }
    }

    pub fn pick_nearest(
        &self,
        world: Vec2,
        max_dist: f32,
        owner: Option<PlayerId>,
    ) -> Option<UnitId> {
        let max2 = max_dist * max_dist;
        let mut best: Option<(UnitId, f32)> = None;
        for u in self.iter() {
            if let Some(o) = owner {
                if u.owner != o {
                    continue;
                }
            }
            let dx = u.pose.pos.x - world.x;
            let dy = u.pose.pos.y - world.y;
            let d2 = dx * dx + dy * dy;
            if d2 <= max2 {
                if best.map(|(_, bd)| d2 < bd).unwrap_or(true) {
                    best = Some((u.id, d2));
                }
            }
        }
        best.map(|(id, _)| id)
    }

    pub fn query_rect(
        &self,
        min: Vec2,
        max: Vec2,
        owner: Option<PlayerId>,
    ) -> Vec<UnitId> {
        let (x0, x1) = (min.x.min(max.x), min.x.max(max.x));
        let (y0, y1) = (min.y.min(max.y), min.y.max(max.y));
        self.iter()
            .filter(|u| {
                if let Some(o) = owner {
                    if u.owner != o {
                        return false;
                    }
                }
                u.pose.pos.x >= x0
                    && u.pose.pos.x <= x1
                    && u.pose.pos.y >= y0
                    && u.pose.pos.y <= y1
            })
            .map(|u| u.id)
            .collect()
    }
}
