//! 2D 仿射变换（平移 + 旋转 + 均匀缩放）。

use spark_types::Vec2;

/// 二维刚体+均匀缩放变换。
///
/// 应用顺序：先缩放，再绕原点旋转 `rotation` 弧度，再平移。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2 {
    /// 平移（世界单位）。
    pub translation: Vec2,
    /// 绕 Z 的旋转角（弧度，逆时针为正）。
    pub rotation: f32,
    /// 均匀缩放因子（负值表示反射）。
    pub scale: f32,
}

impl Default for Transform2 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform2 {
    /// 单位变换（无平移、无旋转、缩放 1）。
    pub const IDENTITY: Self = Self { translation: Vec2::ZERO, rotation: 0.0, scale: 1.0 };

    /// 构造。
    pub const fn new(translation: Vec2, rotation: f32, scale: f32) -> Self {
        Self { translation, rotation, scale }
    }

    /// 变换点（含平移）。
    pub fn transform_point(self, p: Vec2) -> Vec2 {
        let s = self.scale;
        let q = Vec2::new(p.x * s, p.y * s).rotate(self.rotation);
        q.add(self.translation)
    }

    /// 变换方向 / 向量（忽略平移，保留缩放与旋转）。
    pub fn transform_dir(self, d: Vec2) -> Vec2 {
        d.mul_scalar(self.scale).rotate(self.rotation)
    }

    /// 逆变换；`|scale| < 1e-8` 时逆缩放取 0（退化）。
    pub fn inverse(self) -> Self {
        let inv_s = if self.scale.abs() < 1e-8 { 0.0 } else { 1.0 / self.scale };
        let inv_r = -self.rotation;
        let t = self.translation.neg().rotate(inv_r).mul_scalar(inv_s);
        Self { translation: t, rotation: inv_r, scale: inv_s }
    }
}
