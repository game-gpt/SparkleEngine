//! 2D 刚体。

use spark_types::{Rect, Vec2};
use spark_geometry::Circle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyKind {
    Dynamic,
    Kinematic,
    Static,
}

#[derive(Debug, Clone)]
pub enum Collider2 {
    Aabb(Rect),
    Circle(Circle),
}

#[derive(Debug, Clone)]
pub struct RigidBody2 {
    pub kind: BodyKind,
    pub position: Vec2,
    pub velocity: Vec2,
    pub collider: Collider2,
    /// 质量；静态体忽略。
    pub mass: f32,
}

impl RigidBody2 {
    pub fn aabb(kind: BodyKind, center: Vec2, half_extents: Vec2) -> Self {
        let rect = Rect::new(center.x - half_extents.x, center.y - half_extents.y, half_extents.x * 2.0, half_extents.y * 2.0);
        Self { kind, position: center, velocity: Vec2::ZERO, collider: Collider2::Aabb(rect), mass: 1.0 }
    }

    pub fn circle(kind: BodyKind, center: Vec2, radius: f32) -> Self {
        Self { kind, position: center, velocity: Vec2::ZERO, collider: Collider2::Circle(Circle::new(center, radius)), mass: 1.0 }
    }

    pub fn sync_collider(&mut self) {
        match &mut self.collider {
            Collider2::Aabb(r) => {
                let hw = r.w * 0.5;
                let hh = r.h * 0.5;
                r.x = self.position.x - hw;
                r.y = self.position.y - hh;
            }
            Collider2::Circle(c) => {
                c.center = self.position;
            }
        }
    }

    pub fn aabb_bounds(&self) -> Rect {
        match &self.collider {
            Collider2::Aabb(r) => *r,
            Collider2::Circle(c) => Rect::new(c.center.x - c.radius, c.center.y - c.radius, c.radius * 2.0, c.radius * 2.0),
        }
    }
}
