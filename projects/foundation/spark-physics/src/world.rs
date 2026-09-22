//! 物理世界步进。

use spark_types::Vec2;
use spark_geometry::{aabb_aabb, circle_aabb, circle_circle};

use crate::{
    body::{BodyId, BodyKind, Collider2, RigidBody2},
    broadphase::{Broadphase, UniformGrid},
};

#[derive(Debug, Clone)]
pub struct PhysicsConfig {
    pub gravity: Vec2,
    pub cell_size: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self { gravity: Vec2::new(0.0, 980.0), cell_size: 64.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contact {
    pub a: BodyId,
    pub b: BodyId,
}

#[derive(Debug)]
pub struct PhysicsWorld {
    pub config: PhysicsConfig,
    bodies: Vec<Option<RigidBody2>>,
    free: Vec<u32>,
    broadphase: UniformGrid,
    contacts: Vec<Contact>,
}

impl PhysicsWorld {
    pub fn new(config: PhysicsConfig) -> Self {
        let cell = config.cell_size;
        Self { config, bodies: Vec::new(), free: Vec::new(), broadphase: UniformGrid::new(cell), contacts: Vec::new() }
    }

    pub fn spawn(&mut self, body: RigidBody2) -> BodyId {
        if let Some(slot) = self.free.pop() {
            self.bodies[slot as usize] = Some(body);
            BodyId(slot)
        }
        else {
            let id = self.bodies.len() as u32;
            self.bodies.push(Some(body));
            BodyId(id)
        }
    }

    pub fn despawn(&mut self, id: BodyId) -> bool {
        let Some(slot) = self.bodies.get_mut(id.0 as usize)
        else {
            return false;
        };
        if slot.take().is_some() {
            self.free.push(id.0);
            true
        }
        else {
            false
        }
    }

    pub fn get(&self, id: BodyId) -> Option<&RigidBody2> {
        self.bodies.get(id.0 as usize).and_then(|b| b.as_ref())
    }

    pub fn get_mut(&mut self, id: BodyId) -> Option<&mut RigidBody2> {
        self.bodies.get_mut(id.0 as usize).and_then(|b| b.as_mut())
    }

    pub fn contacts(&self) -> &[Contact] {
        &self.contacts
    }

    pub fn body_count(&self) -> usize {
        self.bodies.iter().filter(|b| b.is_some()).count()
    }

    /// 积分速度/位置，重建宽相并收集窄相接触（不做冲量求解，留给后续迭代）。
    pub fn step(&mut self, dt: f32) {
        let g = self.config.gravity;
        for body in self.bodies.iter_mut().flatten() {
            if body.kind == BodyKind::Dynamic {
                body.velocity.x += g.x * dt;
                body.velocity.y += g.y * dt;
                body.position.x += body.velocity.x * dt;
                body.position.y += body.velocity.y * dt;
                body.sync_collider();
            }
            else if body.kind == BodyKind::Kinematic {
                body.position.x += body.velocity.x * dt;
                body.position.y += body.velocity.y * dt;
                body.sync_collider();
            }
            else {
                body.sync_collider();
            }
        }

        self.broadphase.clear();
        let mut ids = Vec::new();
        for (i, slot) in self.bodies.iter().enumerate() {
            if let Some(body) = slot {
                let id = BodyId(i as u32);
                self.broadphase.insert(id, body.aabb_bounds());
                ids.push(id);
            }
        }

        self.contacts.clear();
        for (a, b) in self.broadphase.query_pairs() {
            if self.narrowphase(a, b) {
                self.contacts.push(Contact { a, b });
            }
        }
    }

    fn narrowphase(&self, a: BodyId, b: BodyId) -> bool {
        let (Some(ba), Some(bb)) = (self.get(a), self.get(b))
        else {
            return false;
        };
        match (&ba.collider, &bb.collider) {
            (Collider2::Aabb(ra), Collider2::Aabb(rb)) => aabb_aabb(*ra, *rb),
            (Collider2::Circle(ca), Collider2::Circle(cb)) => circle_circle(*ca, *cb),
            (Collider2::Circle(c), Collider2::Aabb(r)) | (Collider2::Aabb(r), Collider2::Circle(c)) => circle_aabb(*c, *r),
        }
    }
}
