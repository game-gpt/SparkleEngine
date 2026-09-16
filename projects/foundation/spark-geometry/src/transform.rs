//! 2D 仿射变换（平移 + 旋转 + 均匀缩放）。

use spark_core::Vec2;

use crate::Vec2Ext;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2 {
    pub translation: Vec2,
    pub rotation: f32,
    pub scale: f32,
}

impl Default for Transform2 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform2 {
    pub const IDENTITY: Self = Self {
        translation: Vec2::ZERO,
        rotation: 0.0,
        scale: 1.0,
    };

    pub const fn new(translation: Vec2, rotation: f32, scale: f32) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    pub fn transform_point(self, p: Vec2) -> Vec2 {
        let s = self.scale;
        let q = Vec2::new(p.x * s, p.y * s).rotate(self.rotation);
        q.add(self.translation)
    }

    pub fn transform_dir(self, d: Vec2) -> Vec2 {
        d.mul_scalar(self.scale).rotate(self.rotation)
    }

    pub fn inverse(self) -> Self {
        let inv_s = if self.scale.abs() < 1e-8 {
            0.0
        } else {
            1.0 / self.scale
        };
        let inv_r = -self.rotation;
        let t = self.translation.neg().rotate(inv_r).mul_scalar(inv_s);
        Self {
            translation: t,
            rotation: inv_r,
            scale: inv_s,
        }
    }
}
