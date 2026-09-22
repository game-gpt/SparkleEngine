//! 线段与射线。

use spark_types::Vec2;


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSegment {
    pub a: Vec2,
    pub b: Vec2,
}

impl LineSegment {
    pub const fn new(a: Vec2, b: Vec2) -> Self {
        Self { a, b }
    }

    pub fn delta(self) -> Vec2 {
        self.b.sub(self.a)
    }

    pub fn length(self) -> f32 {
        self.delta().length()
    }

    pub fn closest_point(self, p: Vec2) -> Vec2 {
        let ab = self.delta();
        let len2 = ab.length_squared();
        if len2 < 1e-12 {
            return self.a;
        }
        let t = (p.sub(self.a).dot(ab) / len2).clamp(0.0, 1.0);
        self.a.add(ab.mul_scalar(t))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Vec2,
    /// 方向（不必单位化）。
    pub dir: Vec2,
}

impl Ray {
    pub const fn new(origin: Vec2, dir: Vec2) -> Self {
        Self { origin, dir }
    }

    pub fn point_at(self, t: f32) -> Vec2 {
        self.origin.add(self.dir.mul_scalar(t))
    }
}
