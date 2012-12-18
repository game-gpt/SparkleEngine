//! 弹幕对象池。

use spark_core::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BulletId(pub u32);

#[derive(Debug, Clone, Copy)]
pub struct Bullet {
    pub id: BulletId,
    pub pos: Vec2,
    pub vel: Vec2,
    pub radius: f32,
    pub layer: u8,
    pub alive: bool,
}

#[derive(Debug)]
pub struct BulletPool {
    next: u32,
    slots: Vec<Bullet>,
}

impl BulletPool {
    pub fn with_capacity(cap: usize) -> Self {
        Self { next: 1, slots: Vec::with_capacity(cap) }
    }

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

    pub fn integrate(&mut self, dt: f32) {
        for b in &mut self.slots {
            if !b.alive {
                continue;
            }
            b.pos.x += b.vel.x * dt;
            b.pos.y += b.vel.y * dt;
        }
    }

    pub fn despawn(&mut self, id: BulletId) {
        if let Some(b) = self.slots.iter_mut().find(|b| b.id == id) {
            b.alive = false;
        }
    }

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

    pub fn iter_alive(&self) -> impl Iterator<Item = &Bullet> {
        self.slots.iter().filter(|b| b.alive)
    }

    pub fn alive_count(&self) -> usize {
        self.slots.iter().filter(|b| b.alive).count()
    }

    pub fn clear(&mut self) {
        for b in &mut self.slots {
            b.alive = false;
        }
    }
}
