//! [`Vec2`] 运算扩展。

use spark_core::Vec2;

/// 为 [`Vec2`] 提供常用 2D 向量运算（不改动 `spark-core` 定义）。
pub trait Vec2Ext: Sized {
    fn length(self) -> f32;
    fn length_squared(self) -> f32;
    fn normalized(self) -> Self;
    fn try_normalized(self) -> Option<Self>;
    fn dot(self, other: Self) -> f32;
    /// 2D 叉积标量：`x1*y2 - y1*x2`。
    fn cross(self, other: Self) -> f32;
    fn lerp(self, other: Self, t: f32) -> Self;
    fn rotate(self, radians: f32) -> Self;
    fn perpendicular(self) -> Self;
    fn distance(self, other: Self) -> f32;
    fn distance_squared(self, other: Self) -> f32;
    fn mul_scalar(self, s: f32) -> Self;
    fn add(self, other: Self) -> Self;
    fn sub(self, other: Self) -> Self;
    fn neg(self) -> Self;
    fn clamp_length(self, max: f32) -> Self;
}

impl Vec2Ext for Vec2 {
    fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    fn normalized(self) -> Self {
        self.try_normalized().unwrap_or(Self::ZERO)
    }

    fn try_normalized(self) -> Option<Self> {
        let len = self.length();
        if len < 1e-8 { None } else { Some(Self::new(self.x / len, self.y / len)) }
    }

    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    fn cross(self, other: Self) -> f32 {
        self.x * other.y - self.y * other.x
    }

    fn lerp(self, other: Self, t: f32) -> Self {
        Self::new(self.x + (other.x - self.x) * t, self.y + (other.y - self.y) * t)
    }

    fn rotate(self, radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        Self::new(self.x * c - self.y * s, self.x * s + self.y * c)
    }

    fn perpendicular(self) -> Self {
        Self::new(-self.y, self.x)
    }

    fn distance(self, other: Self) -> f32 {
        self.sub(other).length()
    }

    fn distance_squared(self, other: Self) -> f32 {
        self.sub(other).length_squared()
    }

    fn mul_scalar(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s)
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }

    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }

    fn clamp_length(self, max: f32) -> Self {
        let max = max.max(0.0);
        let d2 = self.length_squared();
        if d2 <= max * max || d2 < 1e-12 { self } else { self.mul_scalar(max / d2.sqrt()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_and_rotate() {
        let v = Vec2::new(3.0, 4.0);
        assert!((v.length() - 5.0).abs() < 1e-5);
        let n = v.normalized();
        assert!((n.length() - 1.0).abs() < 1e-5);
        let r = Vec2::new(1.0, 0.0).rotate(std::f32::consts::FRAC_PI_2);
        assert!(r.x.abs() < 1e-5);
        assert!((r.y - 1.0).abs() < 1e-5);
    }
}
