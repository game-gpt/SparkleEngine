//! 列主序 4×4 矩阵（与 WGSL `mat4x4` 内存布局一致）。

use crate::{Quat, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    /// 列主序 16 浮点。
    pub cols: [f32; 16],
}

impl Mat4 {
    pub const IDENTITY: Self = Self {
        cols: [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ],
    };
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mat4 {
    pub fn from_cols(cols: [f32; 16]) -> Self {
        Self { cols }
    }

    pub fn translation(t: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.cols[12] = t.x;
        m.cols[13] = t.y;
        m.cols[14] = t.z;
        m
    }

    pub fn scaling(s: Vec3) -> Self {
        Self {
            cols: [
                s.x, 0.0, 0.0, 0.0, //
                0.0, s.y, 0.0, 0.0, //
                0.0, 0.0, s.z, 0.0, //
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    /// `T * R * S`（先缩放，再旋转，再平移）。
    pub fn from_trs(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self::translation(translation).mul(rotation.to_mat4()).mul(Self::scaling(scale))
    }

    pub fn rotation_y(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self {
            cols: [
                c, 0.0, -s, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                s, 0.0, c, 0.0, //
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn rotation_x(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self {
            cols: [
                1.0, 0.0, 0.0, 0.0, //
                0.0, c, s, 0.0, //
                0.0, -s, c, 0.0, //
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn perspective(fov_y_rad: f32, aspect: f32, near: f32, far: f32) -> Self {
        // WebGPU / wgpu：裁剪空间 Z ∈ [0, 1]，Y 向上。
        let f = 1.0 / (fov_y_rad * 0.5).tan();
        let mut cols = [0.0; 16];
        cols[0] = f / aspect;
        cols[5] = f;
        cols[10] = far / (near - far);
        cols[11] = -1.0;
        cols[14] = (far * near) / (near - far);
        Self { cols }
    }

    /// 正交投影（WebGPU Z ∈ [0, 1]，Y 向上）。用于阴影图 / 2.5D。
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let mut cols = [0.0; 16];
        let rl = (right - left).max(1e-6);
        let tb = (top - bottom).max(1e-6);
        let fn_ = (far - near).max(1e-6);
        cols[0] = 2.0 / rl;
        cols[5] = 2.0 / tb;
        cols[10] = 1.0 / fn_;
        cols[12] = -(right + left) / rl;
        cols[13] = -(top + bottom) / tb;
        cols[14] = -near / fn_;
        cols[15] = 1.0;
        Self { cols }
    }

    pub fn look_to(eye: Vec3, forward: Vec3, up: Vec3) -> Self {
        let f = forward.normalized();
        let mut s = f.cross(up);
        if s.length() < 1e-6 {
            // 俯仰近 ±90° 时与 up 共线，换备用轴。
            s = f.cross(Vec3::X);
            if s.length() < 1e-6 {
                s = f.cross(Vec3::Z);
            }
        }
        let s = s.normalized();
        let u = s.cross(f);
        let mut cols = [0.0; 16];
        cols[0] = s.x;
        cols[4] = s.y;
        cols[8] = s.z;
        cols[1] = u.x;
        cols[5] = u.y;
        cols[9] = u.z;
        cols[2] = -f.x;
        cols[6] = -f.y;
        cols[10] = -f.z;
        cols[12] = -s.dot(eye);
        cols[13] = -u.dot(eye);
        cols[14] = f.dot(eye);
        cols[15] = 1.0;
        Self { cols }
    }

    pub fn mul(self, rhs: Self) -> Self {
        let mut out = [0.0; 16];
        for col in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.cols[k * 4 + row] * rhs.cols[col * 4 + k];
                }
                out[col * 4 + row] = sum;
            }
        }
        Self { cols: out }
    }

    pub fn transform_point(self, p: Vec3) -> Vec3 {
        let x = self.cols[0] * p.x + self.cols[4] * p.y + self.cols[8] * p.z + self.cols[12];
        let y = self.cols[1] * p.x + self.cols[5] * p.y + self.cols[9] * p.z + self.cols[13];
        let z = self.cols[2] * p.x + self.cols[6] * p.y + self.cols[10] * p.z + self.cols[14];
        let w = self.cols[3] * p.x + self.cols[7] * p.y + self.cols[11] * p.z + self.cols[15];
        if w.abs() > 1e-8 { Vec3::new(x / w, y / w, z / w) } else { Vec3::new(x, y, z) }
    }

    /// 变换方向（忽略平移，`w=0`）。
    pub fn transform_vector(self, v: Vec3) -> Vec3 {
        Vec3::new(
            self.cols[0] * v.x + self.cols[4] * v.y + self.cols[8] * v.z,
            self.cols[1] * v.x + self.cols[5] * v.y + self.cols[9] * v.z,
            self.cols[2] * v.x + self.cols[6] * v.y + self.cols[10] * v.z,
        )
    }

    /// 一般 4×4 逆；奇异时返回 `None`。
    ///
    /// # 不变式
    /// 用伴随矩阵 / 行列式；`|det| < 1e-12` 视为不可逆。
    pub fn try_inverse(self) -> Option<Self> {
        let m = &self.cols;
        let mut inv = [0.0f32; 16];

        inv[0] = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] - m[9] * m[6] * m[15] + m[9] * m[7] * m[14] + m[13] * m[6] * m[11]
            - m[13] * m[7] * m[10];
        inv[4] = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] + m[8] * m[6] * m[15] - m[8] * m[7] * m[14] - m[12] * m[6] * m[11]
            + m[12] * m[7] * m[10];
        inv[8] =
            m[4] * m[9] * m[15] - m[4] * m[11] * m[13] - m[8] * m[5] * m[15] + m[8] * m[7] * m[13] + m[12] * m[5] * m[11] - m[12] * m[7] * m[9];
        inv[12] = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] + m[8] * m[5] * m[14] - m[8] * m[6] * m[13] - m[12] * m[5] * m[10]
            + m[12] * m[6] * m[9];
        inv[1] = -m[1] * m[10] * m[15] + m[1] * m[11] * m[14] + m[9] * m[2] * m[15] - m[9] * m[3] * m[14] - m[13] * m[2] * m[11]
            + m[13] * m[3] * m[10];
        inv[5] = m[0] * m[10] * m[15] - m[0] * m[11] * m[14] - m[8] * m[2] * m[15] + m[8] * m[3] * m[14] + m[12] * m[2] * m[11]
            - m[12] * m[3] * m[10];
        inv[9] = -m[0] * m[9] * m[15] + m[0] * m[11] * m[13] + m[8] * m[1] * m[15] - m[8] * m[3] * m[13] - m[12] * m[1] * m[11]
            + m[12] * m[3] * m[9];
        inv[13] =
            m[0] * m[9] * m[14] - m[0] * m[10] * m[13] - m[8] * m[1] * m[14] + m[8] * m[2] * m[13] + m[12] * m[1] * m[10] - m[12] * m[2] * m[9];
        inv[2] =
            m[1] * m[6] * m[15] - m[1] * m[7] * m[14] - m[5] * m[2] * m[15] + m[5] * m[3] * m[14] + m[13] * m[2] * m[7] - m[13] * m[3] * m[6];
        inv[6] =
            -m[0] * m[6] * m[15] + m[0] * m[7] * m[14] + m[4] * m[2] * m[15] - m[4] * m[3] * m[14] - m[12] * m[2] * m[7] + m[12] * m[3] * m[6];
        inv[10] =
            m[0] * m[5] * m[15] - m[0] * m[7] * m[13] - m[4] * m[1] * m[15] + m[4] * m[3] * m[13] + m[12] * m[1] * m[7] - m[12] * m[3] * m[5];
        inv[14] =
            -m[0] * m[5] * m[14] + m[0] * m[6] * m[13] + m[4] * m[1] * m[14] - m[4] * m[2] * m[13] - m[12] * m[1] * m[6] + m[12] * m[2] * m[5];
        inv[3] =
            -m[1] * m[6] * m[11] + m[1] * m[7] * m[10] + m[5] * m[2] * m[11] - m[5] * m[3] * m[10] - m[9] * m[2] * m[7] + m[9] * m[3] * m[6];
        inv[7] =
            m[0] * m[6] * m[11] - m[0] * m[7] * m[10] - m[4] * m[2] * m[11] + m[4] * m[3] * m[10] + m[8] * m[2] * m[7] - m[8] * m[3] * m[6];
        inv[11] =
            -m[0] * m[5] * m[11] + m[0] * m[7] * m[9] + m[4] * m[1] * m[11] - m[4] * m[3] * m[9] - m[8] * m[1] * m[7] + m[8] * m[3] * m[5];
        inv[15] = m[0] * m[5] * m[10] - m[0] * m[6] * m[9] - m[4] * m[1] * m[10] + m[4] * m[2] * m[9] + m[8] * m[1] * m[6] - m[8] * m[2] * m[5];

        let det = m[0] * inv[0] + m[1] * inv[4] + m[2] * inv[8] + m[3] * inv[12];
        if det.abs() < 1e-12 {
            return None;
        }
        let inv_det = 1.0 / det;
        for v in &mut inv {
            *v *= inv_det;
        }
        Some(Self { cols: inv })
    }

    pub fn as_cols(&self) -> &[f32; 16] {
        &self.cols
    }
}
