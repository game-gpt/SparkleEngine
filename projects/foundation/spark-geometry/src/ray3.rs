//! 三维射线与体素 DDA / AABB 相交。

use crate::{Aabb3, Vec3};

/// 三维射线。`dir` 不必单位化，但 DDA / 距离判定会先归一化。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray3 {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray3 {
    pub const fn new(origin: Vec3, dir: Vec3) -> Self {
        Self { origin, dir }
    }

    pub fn point_at(self, t: f32) -> Vec3 {
        self.origin + self.dir * t
    }

    pub fn normalized_dir(self) -> Self {
        Self {
            origin: self.origin,
            dir: self.dir.normalized(),
        }
    }
}

/// 体素格命中（无游戏方块语义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxelHit {
    pub cell: [i32; 3],
    /// 进入该格前一格（放置面）。
    pub prev: [i32; 3],
}

/// 射线与 AABB 相交。返回进入距离 `t`（沿**单位化**方向），未命中为 `None`。
pub fn ray_aabb(ray: Ray3, aabb: Aabb3) -> Option<f32> {
    let dir = ray.dir.normalized();
    if dir.length() < 1e-8 {
        return if aabb.contains_point(ray.origin) {
            Some(0.0)
        } else {
            None
        };
    }

    let inv = Vec3::new(
        if dir.x.abs() < 1e-8 {
            f32::INFINITY.copysign(dir.x)
        } else {
            1.0 / dir.x
        },
        if dir.y.abs() < 1e-8 {
            f32::INFINITY.copysign(dir.y)
        } else {
            1.0 / dir.y
        },
        if dir.z.abs() < 1e-8 {
            f32::INFINITY.copysign(dir.z)
        } else {
            1.0 / dir.z
        },
    );

    let mut t1 = (aabb.min.x - ray.origin.x) * inv.x;
    let mut t2 = (aabb.max.x - ray.origin.x) * inv.x;
    let mut tmin = t1.min(t2);
    let mut tmax = t1.max(t2);

    t1 = (aabb.min.y - ray.origin.y) * inv.y;
    t2 = (aabb.max.y - ray.origin.y) * inv.y;
    tmin = tmin.max(t1.min(t2));
    tmax = tmax.min(t1.max(t2));

    t1 = (aabb.min.z - ray.origin.z) * inv.z;
    t2 = (aabb.max.z - ray.origin.z) * inv.z;
    tmin = tmin.max(t1.min(t2));
    tmax = tmax.min(t1.max(t2));

    if tmax >= tmin.max(0.0) {
        Some(tmin.max(0.0))
    } else {
        None
    }
}

fn int_bound(s: f32, ds: f32) -> f32 {
    if ds.abs() < 1e-8 {
        return f32::INFINITY;
    }
    if ds > 0.0 {
        (s.ceil() - s) / ds
    } else {
        (s - s.floor()) / (-ds)
    }
}

/// 单位体素格 DDA。`solid(x,y,z)` 为真则命中。
///
/// 无游戏语义：调用方自行解释实心判定（方块表、体积掩码等）。
pub fn ray_voxel_dda(
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
    max_steps: u32,
    mut solid: impl FnMut(i32, i32, i32) -> bool,
) -> Option<VoxelHit> {
    let dir = dir.normalized();
    if dir.length() < 1e-6 {
        return None;
    }

    let mut x = origin.x.floor() as i32;
    let mut y = origin.y.floor() as i32;
    let mut z = origin.z.floor() as i32;

    let step_x = if dir.x > 0.0 { 1 } else { -1 };
    let step_y = if dir.y > 0.0 { 1 } else { -1 };
    let step_z = if dir.z > 0.0 { 1 } else { -1 };

    let t_delta_x = if dir.x.abs() < 1e-8 {
        f32::INFINITY
    } else {
        (1.0 / dir.x).abs()
    };
    let t_delta_y = if dir.y.abs() < 1e-8 {
        f32::INFINITY
    } else {
        (1.0 / dir.y).abs()
    };
    let t_delta_z = if dir.z.abs() < 1e-8 {
        f32::INFINITY
    } else {
        (1.0 / dir.z).abs()
    };

    let mut t_max_x = int_bound(origin.x, dir.x);
    let mut t_max_y = int_bound(origin.y, dir.y);
    let mut t_max_z = int_bound(origin.z, dir.z);

    let mut prev = [x, y, z];

    for _ in 0..max_steps {
        if solid(x, y, z) {
            return Some(VoxelHit {
                cell: [x, y, z],
                prev,
            });
        }
        prev = [x, y, z];
        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                if t_max_x > max_dist {
                    break;
                }
                x += step_x;
                t_max_x += t_delta_x;
            } else {
                if t_max_z > max_dist {
                    break;
                }
                z += step_z;
                t_max_z += t_delta_z;
            }
        } else if t_max_y < t_max_z {
            if t_max_y > max_dist {
                break;
            }
            y += step_y;
            t_max_y += t_delta_y;
        } else {
            if t_max_z > max_dist {
                break;
            }
            z += step_z;
            t_max_z += t_delta_z;
        }
    }
    None
}

/// [`Ray3`] 封装的体素 DDA。
pub fn ray_voxel(ray: Ray3, max_dist: f32, max_steps: u32, solid: impl FnMut(i32, i32, i32) -> bool) -> Option<VoxelHit> {
    ray_voxel_dda(ray.origin, ray.dir, max_dist, max_steps, solid)
}
