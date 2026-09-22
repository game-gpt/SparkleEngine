//! 单位四元数（x, y, z, w），列主序矩阵约定下的旋转。

use crate::{Mat4, Vec3};

/// 旋转四元数。插值前应归一化；恒等为 `(0,0,0,1)`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub const IDENTITY: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn from_axis_angle(axis: Vec3, rad: f32) -> Self {
        let axis = axis.normalized();
        let (s, c) = (rad * 0.5).sin_cos();
        Self { x: axis.x * s, y: axis.y * s, z: axis.z * s, w: c }
    }

    /// 最短弧旋转：把单位向量 `from` 转到 `to`。
    pub fn rotation_between(from: Vec3, to: Vec3) -> Self {
        let a = from.normalized();
        let b = to.normalized();
        let dot = a.dot(b).clamp(-1.0, 1.0);
        if dot > 0.999999 {
            return Self::IDENTITY;
        }
        if dot < -0.999999 {
            // 反向：绕与 a 垂直的轴转 π
            let mut axis = Vec3::X.cross(a);
            if axis.length() < 1e-6 {
                axis = Vec3::Y.cross(a);
            }
            return Self::from_axis_angle(axis.normalized(), std::f32::consts::PI);
        }
        let axis = a.cross(b);
        let s = ((1.0 + dot) * 2.0).sqrt();
        let inv_s = 1.0 / s;
        Self { x: axis.x * inv_s, y: axis.y * inv_s, z: axis.z * inv_s, w: s * 0.5 }.normalized()
    }

    /// YXZ 欧拉角（弧度）：先 yaw(Y)，再 pitch(X)，再 roll(Z)。
    pub fn from_euler_yxz(yaw: f32, pitch: f32, roll: f32) -> Self {
        let qy = Self::from_axis_angle(Vec3::Y, yaw);
        let qx = Self::from_axis_angle(Vec3::X, pitch);
        let qz = Self::from_axis_angle(Vec3::Z, roll);
        qy.mul(qx).mul(qz)
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt()
    }

    pub fn normalized(self) -> Self {
        let len = self.length();
        if len <= 1e-8 { Self::IDENTITY } else { Self { x: self.x / len, y: self.y / len, z: self.z / len, w: self.w / len } }
    }

    pub fn conjugate(self) -> Self {
        Self { x: -self.x, y: -self.y, z: -self.z, w: self.w }
    }

    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z + self.w * o.w
    }

    /// 四元数乘法：先应用 `rhs`，再应用 `self`（与矩阵左乘一致）。
    pub fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
        }
    }

    pub fn rotate_vec3(self, v: Vec3) -> Vec3 {
        let q = self.normalized();
        let u = Vec3::new(q.x, q.y, q.z);
        let s = q.w;
        // t = 2 * cross(u, v)
        let t = u.cross(v) * 2.0;
        // v + s*t + cross(u, t)
        v + t * s + u.cross(t)
    }

    /// 归一化线性插值后归一化（短弧）。
    pub fn nlerp(self, other: Self, t: f32) -> Self {
        let mut b = other;
        if self.dot(b) < 0.0 {
            b = Self { x: -b.x, y: -b.y, z: -b.z, w: -b.w };
        }
        Self { x: self.x + (b.x - self.x) * t, y: self.y + (b.y - self.y) * t, z: self.z + (b.z - self.z) * t, w: self.w + (b.w - self.w) * t }
            .normalized()
    }

    /// 球面线性插值（短弧）。
    pub fn slerp(self, other: Self, t: f32) -> Self {
        let a = self.normalized();
        let mut b = other.normalized();
        let mut dot = a.dot(b);
        if dot < 0.0 {
            b = Self { x: -b.x, y: -b.y, z: -b.z, w: -b.w };
            dot = -dot;
        }
        if dot > 0.9995 {
            return a.nlerp(b, t);
        }
        let theta = dot.clamp(-1.0, 1.0).acos();
        let sin_theta = theta.sin();
        let w1 = ((1.0 - t) * theta).sin() / sin_theta;
        let w2 = (t * theta).sin() / sin_theta;
        Self { x: a.x * w1 + b.x * w2, y: a.y * w1 + b.y * w2, z: a.z * w1 + b.z * w2, w: a.w * w1 + b.w * w2 }.normalized()
    }

    pub fn to_mat4(self) -> Mat4 {
        let q = self.normalized();
        let (x, y, z, w) = (q.x, q.y, q.z, q.w);
        let x2 = x + x;
        let y2 = y + y;
        let z2 = z + z;
        let xx = x * x2;
        let xy = x * y2;
        let xz = x * z2;
        let yy = y * y2;
        let yz = y * z2;
        let zz = z * z2;
        let wx = w * x2;
        let wy = w * y2;
        let wz = w * z2;
        Mat4::from_cols([
            1.0 - (yy + zz),
            xy + wz,
            xz - wy,
            0.0,
            xy - wz,
            1.0 - (xx + zz),
            yz + wx,
            0.0,
            xz + wy,
            yz - wx,
            1.0 - (xx + yy),
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        ])
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }
}

impl Default for Quat {
    fn default() -> Self {
        Self::IDENTITY
    }
}
