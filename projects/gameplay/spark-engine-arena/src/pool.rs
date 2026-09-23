//! 动能体对象池（弹体、敌人、拾取物等）。

use spark_types::Vec2;

/// 池内稳定句柄。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyId(pub u32);

/// 带速度的运行时刚体点质量近似（圆形判定）。
#[derive(Debug, Clone, Copy)]
pub struct KineticBody {
    /// 句柄。
    pub id: BodyId,
    /// 位置。
    pub pos: Vec2,
    /// 速度（单位/秒）。
    pub vel: Vec2,
    /// 碰撞半径。
    pub radius: f32,
    /// 游戏自定义标签（敌种、弹种等）。
    pub tag: u32,
    /// 碰撞/渲染层。
    pub layer: u8,
    /// 是否存活。
    pub alive: bool,
}

/// 定容或可增长槽数组。
#[derive(Debug)]
pub struct KineticPool {
    next: u32,
    slots: Vec<KineticBody>,
    hard_cap: usize,
}

impl KineticPool {
    /// `cap == 0` 表示不设硬上限（可一直 push）。
    pub fn with_capacity(cap: usize) -> Self {
        Self { next: 1, slots: Vec::with_capacity(cap), hard_cap: cap }
    }

    /// 生成实体；无空闲且达硬上限时返回 `None`。
    pub fn spawn(&mut self, pos: Vec2, vel: Vec2, radius: f32, tag: u32, layer: u8) -> Option<BodyId> {
        if let Some(slot) = self.slots.iter_mut().find(|b| !b.alive) {
            let id = slot.id;
            *slot = KineticBody { id, pos, vel, radius, tag, layer, alive: true };
            return Some(id);
        }
        if self.hard_cap > 0 && self.slots.len() >= self.hard_cap {
            return None;
        }
        let id = BodyId(self.next);
        self.next = self.next.saturating_add(1);
        self.slots.push(KineticBody { id, pos, vel, radius, tag, layer, alive: true });
        Some(id)
    }

    /// 速度积分。
    pub fn integrate(&mut self, dt: f32) {
        for b in &mut self.slots {
            if !b.alive {
                continue;
            }
            b.pos.x += b.vel.x * dt;
            b.pos.y += b.vel.y * dt;
        }
    }

    /// 标记死亡。
    pub fn despawn(&mut self, id: BodyId) {
        if let Some(b) = self.slots.iter_mut().find(|b| b.id == id) {
            b.alive = false;
        }
    }

    /// 轴对齐框外 despawn。
    pub fn despawn_out_of_bounds(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        for b in &mut self.slots {
            if !b.alive {
                continue;
            }
            if b.pos.x < x0 || b.pos.y < y0 || b.pos.x > x1 || b.pos.y > y1 {
                b.alive = false;
            }
        }
    }

    /// 存活迭代。
    pub fn iter_alive(&self) -> impl Iterator<Item = &KineticBody> {
        self.slots.iter().filter(|b| b.alive)
    }

    /// 可变存活迭代。
    pub fn iter_alive_mut(&mut self) -> impl Iterator<Item = &mut KineticBody> {
        self.slots.iter_mut().filter(|b| b.alive)
    }

    /// 按 ID 可变访问。
    pub fn get_mut(&mut self, id: BodyId) -> Option<&mut KineticBody> {
        self.slots.iter_mut().find(|b| b.id == id && b.alive)
    }

    /// 存活数量。
    pub fn alive_count(&self) -> usize {
        self.slots.iter().filter(|b| b.alive).count()
    }

    /// 全部标记死亡。
    pub fn clear(&mut self) {
        for b in &mut self.slots {
            b.alive = false;
        }
    }
}
