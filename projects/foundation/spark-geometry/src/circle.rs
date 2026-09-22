//! 圆。

use spark_types::Vec2;


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}

impl Circle {
    pub const fn new(center: Vec2, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn contains(&self, p: Vec2) -> bool {
        self.center.distance_squared(p) <= self.radius * self.radius
    }

    pub fn expanded(self, by: f32) -> Self {
        Self { center: self.center, radius: (self.radius + by).max(0.0) }
    }
}
