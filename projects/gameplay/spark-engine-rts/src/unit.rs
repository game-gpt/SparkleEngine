//! 单位编制（无兵种数据表）。

use spark_types::Vec2;

/// 玩家 / 控制方不透明 ID（游戏映射阵营）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(pub u32);

/// 单位句柄。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitId(pub u32);

/// 单位位姿：平面位置 + 朝向角（弧度，约定由游戏解释）。
#[derive(Debug, Clone, Copy)]
pub struct UnitPose {
    /// 世界平面坐标。
    pub pos: Vec2,
    /// 朝向角（弧度）。
    pub facing: f32,
}

impl UnitPose {
    /// 置于 `(x, y)`，朝向 `0`。
    pub fn at(x: f32, y: f32) -> Self {
        Self { pos: Vec2::new(x, y), facing: 0.0 }
    }
}

/// 编制中的存活单位快照（迷雾 / 选取查询用）。
#[derive(Debug, Clone)]
pub struct Unit {
    /// 句柄。
    pub id: UnitId,
    /// 所属玩家。
    pub owner: PlayerId,
    /// 当前位姿。
    pub pose: UnitPose,
    /// 视野半径（与 `FogGrid` 采样一致的世界单位）。
    pub sight_radius: f32,
    /// 是否仍计入编制（`despawn` 后为假，句柄不复用）。
    pub alive: bool,
}

/// 单位表：分配单调 ID，软删除保留槽位。
#[derive(Debug, Default)]
pub struct UnitRoster {
    next: u32,
    units: Vec<Unit>,
}

impl UnitRoster {
    /// 空编制。
    pub fn new() -> Self {
        Self::default()
    }

    /// 生成单位并返回新句柄；`sight_radius` 供迷雾贡献。
    pub fn spawn(&mut self, owner: PlayerId, pose: UnitPose, sight_radius: f32) -> UnitId {
        let id = UnitId(self.next);
        self.next = self.next.saturating_add(1);
        self.units.push(Unit { id, owner, pose, sight_radius, alive: true });
        id
    }

    /// 按句柄取存活单位。
    pub fn get(&self, id: UnitId) -> Option<&Unit> {
        self.units.iter().find(|u| u.id == id && u.alive)
    }

    /// 按句柄可变借用存活单位。
    pub fn get_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.units.iter_mut().find(|u| u.id == id && u.alive)
    }

    /// 迭代全部存活单位。
    pub fn iter(&self) -> impl Iterator<Item = &Unit> {
        self.units.iter().filter(|u| u.alive)
    }

    /// 软删除：标记 `alive = false`，不回收 ID。
    pub fn despawn(&mut self, id: UnitId) {
        if let Some(u) = self.units.iter_mut().find(|u| u.id == id) {
            u.alive = false;
        }
    }

    /// 在 `max_dist` 内取最近单位；`owner` 为 `Some` 时只匹配该玩家。
    pub fn pick_nearest(&self, world: Vec2, max_dist: f32, owner: Option<PlayerId>) -> Option<UnitId> {
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

    /// 轴对齐矩形内的存活单位（含边界）；可选按 `owner` 过滤。
    pub fn query_rect(&self, min: Vec2, max: Vec2, owner: Option<PlayerId>) -> Vec<UnitId> {
        let (x0, x1) = (min.x.min(max.x), min.x.max(max.x));
        let (y0, y1) = (min.y.min(max.y), min.y.max(max.y));
        self.iter()
            .filter(|u| {
                if let Some(o) = owner {
                    if u.owner != o {
                        return false;
                    }
                }
                u.pose.pos.x >= x0 && u.pose.pos.x <= x1 && u.pose.pos.y >= y0 && u.pose.pos.y <= y1
            })
            .map(|u| u.id)
            .collect()
    }
}
