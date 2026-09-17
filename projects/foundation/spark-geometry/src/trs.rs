//! 平移 / 旋转 / 缩放组合（列主序，父×子）。

use crate::{Mat4, Quat, Vec3};

/// 局部 TRS。组合顺序：`T * R * S`（先缩放，再旋转，再平移）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trs {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Trs {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    };

    pub fn new(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    pub fn from_translation(t: Vec3) -> Self {
        Self {
            translation: t,
            ..Self::IDENTITY
        }
    }

    pub fn to_mat4(self) -> Mat4 {
        Mat4::from_trs(self.translation, self.rotation, self.scale)
    }

    /// 按权重线性混合平移/缩放，旋转用 nlerp。
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            translation: self.translation + (other.translation - self.translation) * t,
            rotation: self.rotation.nlerp(other.rotation, t),
            scale: self.scale + (other.scale - self.scale) * t,
        }
    }
}

impl Default for Trs {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_mat_is_identity() {
        let m = Trs::IDENTITY.to_mat4();
        for i in 0..16 {
            let expected = if i % 5 == 0 { 1.0 } else { 0.0 };
            assert!((m.cols[i] - expected).abs() < 1e-6);
        }
    }

    #[test]
    fn translation_only() {
        let m = Trs::from_translation(Vec3::new(1.0, 2.0, 3.0)).to_mat4();
        let p = m.transform_point(Vec3::ZERO);
        assert!((p.x - 1.0).abs() < 1e-5);
        assert!((p.y - 2.0).abs() < 1e-5);
        assert!((p.z - 3.0).abs() < 1e-5);
    }
}
