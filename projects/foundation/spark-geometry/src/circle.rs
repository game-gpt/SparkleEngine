//! 圆。

use spark_types::Vec2;

/// 二维圆（圆心 + 半径；半径单位与 `Vec2` 一致）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    /// 圆心。
    pub center: Vec2,
    /// 半径（非负语义由调用方保证；[`expanded`] 会钳到 `≥ 0`）。
    pub radius: f32,
}

impl Circle {
    /// 构造；不校验半径符号。
    pub const fn new(center: Vec2, radius: f32) -> Self {
        Self { center, radius }
    }

    /// 点是否在圆盘内（含边界）。
    pub fn contains(&self, p: Vec2) -> bool {
        self.center.distance_squared(p) <= self.radius * self.radius
    }

    /// 半径增加 `by` 后的圆；结果半径钳到 `≥ 0`。
    pub fn expanded(self, by: f32) -> Self {
        Self { center: self.center, radius: (self.radius + by).max(0.0) }
    }
}
