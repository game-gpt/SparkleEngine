//! 线段与射线。

use spark_types::Vec2;

/// 二维有限线段（闭区间端点）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSegment {
    /// 起点。
    pub a: Vec2,
    /// 终点。
    pub b: Vec2,
}

impl LineSegment {
    /// 由两端点构造。
    pub const fn new(a: Vec2, b: Vec2) -> Self {
        Self { a, b }
    }

    /// 向量 `b - a`。
    pub fn delta(self) -> Vec2 {
        self.b.sub(self.a)
    }

    /// 线段长度。
    pub fn length(self) -> f32 {
        self.delta().length()
    }

    /// 点到线段的最近点（投影钳在 `[0,1]`；退化线段返回 `a`）。
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

/// 二维射线：`origin + t * dir`，`t ≥ 0`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    /// 起点。
    pub origin: Vec2,
    /// 方向（不必单位化）。
    pub dir: Vec2,
}

impl Ray {
    /// 构造；`dir` 为零时后续求交通常返回 `None`。
    pub const fn new(origin: Vec2, dir: Vec2) -> Self {
        Self { origin, dir }
    }

    /// 参数 `t` 处的点（`dir` 未单位化时 `t` 按实际长度度量）。
    pub fn point_at(self, t: f32) -> Vec2 {
        self.origin.add(self.dir.mul_scalar(t))
    }
}
