//! 透视视锥（由 view-proj 提取平面，无游戏语义）。

use spark_geometry::{Aabb3, Mat4, Vec3};

#[derive(Debug, Clone, Copy)]
struct Plane {
    n: Vec3,
    d: f32,
}

impl Plane {
    fn normalize(self) -> Self {
        let len = self.n.length();
        if len < 1e-8 { self } else { Self { n: self.n * (1.0 / len), d: self.d / len } }
    }

    fn signed_distance(self, p: Vec3) -> f32 {
        self.n.dot(p) + self.d
    }
}

/// 六个裁剪平面（内向为正）。
#[derive(Debug, Clone)]
pub struct Frustum {
    planes: [Plane; 6],
}

impl Frustum {
    /// 从列主序 `view * proj` 或 `proj * view` 矩阵提取平面（wgpu Z∈[0,1] 可用）。
    pub fn from_view_proj(vp: &Mat4) -> Self {
        let m = &vp.cols;
        // 列主序：列 i 行 j → m[i*4+j]
        let row = |r: usize| -> (f32, f32, f32, f32) { (m[r], m[4 + r], m[8 + r], m[12 + r]) };
        let (r0x, r0y, r0z, r0w) = row(0);
        let (r1x, r1y, r1z, r1w) = row(1);
        let (r2x, r2y, r2z, r2w) = row(2);
        let (r3x, r3y, r3z, r3w) = row(3);

        let mk = |nx: f32, ny: f32, nz: f32, d: f32| Plane { n: Vec3::new(nx, ny, nz), d };

        let planes = [
            mk(r3x + r0x, r3y + r0y, r3z + r0z, r3w + r0w).normalize(), // left
            mk(r3x - r0x, r3y - r0y, r3z - r0z, r3w - r0w).normalize(), // right
            mk(r3x + r1x, r3y + r1y, r3z + r1z, r3w + r1w).normalize(), // bottom
            mk(r3x - r1x, r3y - r1y, r3z - r1z, r3w - r1w).normalize(), // top
            mk(r3x + r2x, r3y + r2y, r3z + r2z, r3w + r2w).normalize(), // near
            mk(r3x - r2x, r3y - r2y, r3z - r2z, r3w - r2w).normalize(), // far
        ];
        Self { planes }
    }

    /// AABB 是否与视锥相交（含边界）。全在任一平面外侧则不可见。
    pub fn intersects_aabb(&self, aabb: &Aabb3) -> bool {
        for p in &self.planes {
            let px = if p.n.x >= 0.0 { aabb.max.x } else { aabb.min.x };
            let py = if p.n.y >= 0.0 { aabb.max.y } else { aabb.min.y };
            let pz = if p.n.z >= 0.0 { aabb.max.z } else { aabb.min.z };
            if p.signed_distance(Vec3::new(px, py, pz)) < 0.0 {
                return false;
            }
        }
        true
    }

    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        for p in &self.planes {
            if p.signed_distance(center) < -radius {
                return false;
            }
        }
        true
    }
}

/// 距离 + 视锥可见性过滤参数。
#[derive(Debug, Clone, Copy)]
pub struct CullParams {
    pub eye: Vec3,
    /// 超过则剔除（世界空间）。`None` 表示不按距离裁。
    pub max_distance: Option<f32>,
}

impl CullParams {
    pub fn new(eye: Vec3) -> Self {
        Self { eye, max_distance: None }
    }

    pub fn with_max_distance(mut self, d: f32) -> Self {
        self.max_distance = Some(d.max(0.0));
        self
    }
}
