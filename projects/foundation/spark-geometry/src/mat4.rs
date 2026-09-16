//! 列主序 4×4 矩阵（与 WGSL `mat4x4` 内存布局一致）。

use crate::Vec3;

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
        if w.abs() > 1e-8 {
            Vec3::new(x / w, y / w, z / w)
        } else {
            Vec3::new(x, y, z)
        }
    }

    pub fn as_cols(&self) -> &[f32; 16] {
        &self.cols
    }
}
