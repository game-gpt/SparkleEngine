//! 弹幕对象池。
//!
//! # 不变式
//!
//! - 槽位可复用：`alive == false` 的槽优先被 [`BulletPool::spawn`] 填回，保留原 [`BulletId`]。
//! - 新建槽时 ID 从 1 起递增（饱和）。
//! - 容量：构造时 `with_capacity(cap)`；`cap > 0` 且已满且无空闲槽时 `spawn` 返回 `None`。

use spark_types::Vec2;

/// 弹幕句柄（池内稳定，复用槽时不变）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BulletId(pub u32);

/// 一发弹幕的运行时状态。
#[derive(Debug, Clone, Copy)]
pub struct Bullet {
    /// 池内句柄。
    pub id: BulletId,
    /// 世界坐标。
    pub pos: Vec2,
    /// 速度（世界单位 / 秒）。
    pub vel: Vec2,
    /// 判定半径。
    pub radius: f32,
    /// 碰撞 / 渲染分层标签（语义由游戏定义）。
    pub layer: u8,
    /// 是否参与积分与碰撞；`false` 表示空闲槽。
    pub alive: bool,
}

/// 定容或可增长的弹幕槽数组。
#[derive(Debug)]
pub struct BulletPool {
    next: u32,
    slots: Vec<Bullet>,
}

impl BulletPool {
    /// 预留 `cap` 个槽；`cap == 0` 时 `Vec` 无容量上限语义（可一直 push）。
    pub fn with_capacity(cap: usize) -> Self {
        Self { next: 1, slots: Vec::with_capacity(cap) }
    }

    /// 生成一发弹。优先复用空闲槽；否则在容量允许时 push。
    ///
    /// 返回 `None` 表示已达硬容量且无空闲槽。
    pub fn spawn(&mut self, pos: Vec2, vel: Vec2, radius: f32, layer: u8) -> Option<BulletId> {
        if let Some(slot) = self.slots.iter_mut().find(|b| !b.alive) {
            let id = slot.id;
            *slot = Bullet { id, pos, vel, radius, layer, alive: true };
            return Some(id);
        }
        if self.slots.capacity() > 0 && self.slots.len() >= self.slots.capacity() {
            return None;
        }
        let id = BulletId(self.next);
        self.next = self.next.saturating_add(1);
        self.slots.push(Bullet { id, pos, vel, radius, layer, alive: true });
        Some(id)
    }

    /// 对存活弹按速度积分位置；`dt` 由调用方保证。
    pub fn integrate(&mut self, dt: f32) {
        for b in &mut self.slots {
            if !b.alive {
                continue;
            }
            b.pos.x += b.vel.x * dt;
            b.pos.y += b.vel.y * dt;
        }
    }

    /// 按 ID 标记为死亡；找不到则为 no-op。
    pub fn despawn(&mut self, id: BulletId) {
        if let Some(b) = self.slots.iter_mut().find(|b| b.id == id) {
            b.alive = false;
        }
    }

    /// 将轴对齐框外的存活弹标记死亡（含边界外）。
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

    /// 迭代当前存活弹。
    pub fn iter_alive(&self) -> impl Iterator<Item = &Bullet> {
        self.slots.iter().filter(|b| b.alive)
    }

    /// 存活弹数量。
    pub fn alive_count(&self) -> usize {
        self.slots.iter().filter(|b| b.alive).count()
    }

    /// 全部标记死亡（保留槽与 ID，不清容量）。
    pub fn clear(&mut self) {
        for b in &mut self.slots {
            b.alive = false;
        }
    }
}
