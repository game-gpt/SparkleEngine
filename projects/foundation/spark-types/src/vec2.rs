//! 二维向量。

use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// 二维向量（逻辑 / 呈现共用基础表示）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    /// X。
    pub x: f32,
    /// Y。
    pub y: f32,
}

impl Vec2 {
    /// 零向量。
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// 构造。
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// 欧氏长度。
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// 长度平方。
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// 单位向量；近零时回退 [`Self::ZERO`]。
    pub fn normalized(self) -> Self {
        self.try_normalized().unwrap_or(Self::ZERO)
    }

    /// 单位向量；近零时 `None`。
    pub fn try_normalized(self) -> Option<Self> {
        let len = self.length();
        if len < 1e-8 {
            None
        } else {
            Some(Self::new(self.x / len, self.y / len))
        }
    }

    /// 点积。
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// 2D 叉积标量：`x1*y2 - y1*x2`。
    pub fn cross(self, other: Self) -> f32 {
        self.x * other.y - self.y * other.x
    }

    /// 线性插值。
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self::new(self.x + (other.x - self.x) * t, self.y + (other.y - self.y) * t)
    }

    /// 绕原点旋转（弧度，逆时针）。
    pub fn rotate(self, radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        Self::new(self.x * c - self.y * s, self.x * s + self.y * c)
    }

    /// 左垂直向量 `(-y, x)`。
    pub fn perpendicular(self) -> Self {
        Self::new(-self.y, self.x)
    }

    /// 到另一点的距离。
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    /// 到另一点的距离平方。
    pub fn distance_squared(self, other: Self) -> f32 {
        (self - other).length_squared()
    }

    /// 按标量缩放。
    pub fn mul_scalar(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s)
    }

    /// 向量加法（亦可写 `a + b`）。
    pub fn add(self, other: Self) -> Self {
        self + other
    }

    /// 向量减法（亦可写 `a - b`）。
    pub fn sub(self, other: Self) -> Self {
        self - other
    }

    /// 取反（亦可写 `-a`）。
    pub fn neg(self) -> Self {
        -self
    }

    /// 将长度钳到 `max`（`max <= 0` 时视为 0）。
    pub fn clamp_length(self, max: f32) -> Self {
        let max = max.max(0.0);
        let d2 = self.length_squared();
        if d2 <= max * max || d2 < 1e-12 {
            self
        } else {
            self.mul_scalar(max / d2.sqrt())
        }
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        self.mul_scalar(rhs)
    }
}

impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}
