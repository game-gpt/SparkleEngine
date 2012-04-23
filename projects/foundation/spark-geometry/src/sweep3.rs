//! 三维 AABB 扫掠（运动学，无物理求解器）。

use crate::{Aabb3, Vec3};

/// 扫掠命中：`t` ∈ [0,1] 为沿位移的进入比例，`normal` 指向障碍外侧（指向运动物体）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepHit {
    pub t: f32,
    pub normal: Vec3,
}

/// 将运动 AABB 沿 `delta` 扫掠，与静止 AABB 求首次撞击。
///
/// 用 Minkowski 和把问题化为「中心点 vs 扩展盒」的 slab 扫描。
/// 起点已重叠时返回 `t = 0`、法线沿 `-delta`。
pub fn aabb_sweep(moving: Aabb3, delta: Vec3, obstacle: Aabb3) -> Option<SweepHit> {
    let ext = moving.extents();
    let expanded = Aabb3 {
        min: obstacle.min - ext,
        max: obstacle.max + ext,
    };
    let origin = moving.center();
    sweep_point_aabb(origin, delta, expanded)
}

fn axis_normal(axis: usize, positive_hit: bool) -> Vec3 {
    // positive_hit：撞上 max 面 → 法线朝 +axis；撞上 min 面 → 法线朝 -axis
    match (axis, positive_hit) {
        (0, true) => Vec3::X,
        (0, false) => Vec3::new(-1.0, 0.0, 0.0),
        (1, true) => Vec3::Y,
        (1, false) => Vec3::new(0.0, -1.0, 0.0),
        (_, true) => Vec3::Z,
        (_, false) => Vec3::new(0.0, 0.0, -1.0),
    }
}

fn sweep_point_aabb(origin: Vec3, delta: Vec3, aabb: Aabb3) -> Option<SweepHit> {
    if delta.length() < 1e-12 {
        return if aabb.contains_point(origin) {
            Some(SweepHit {
                t: 0.0,
                normal: Vec3::Y,
            })
        } else {
            None
        };
    }

    let mut t_enter = 0.0f32;
    let mut t_leave = 1.0f32;
    let mut normal = Vec3::ZERO;

    for axis in 0..3 {
        let (o, d, min_v, max_v) = match axis {
            0 => (origin.x, delta.x, aabb.min.x, aabb.max.x),
            1 => (origin.y, delta.y, aabb.min.y, aabb.max.y),
            _ => (origin.z, delta.z, aabb.min.z, aabb.max.z),
        };

        if d.abs() < 1e-12 {
            if o < min_v || o > max_v {
                return None;
            }
            continue;
        }

        let inv = 1.0 / d;
        let mut t1 = (min_v - o) * inv;
        let mut t2 = (max_v - o) * inv;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
        }
        // t1 = 进入。撞 min 面（d>0）法线 -axis；撞 max 面（d<0）法线 +axis
        if t1 > t_enter {
            t_enter = t1;
            normal = if d > 0.0 {
                axis_normal(axis, false)
            } else {
                axis_normal(axis, true)
            };
        }
        t_leave = t_leave.min(t2);
        if t_enter > t_leave {
            return None;
        }
    }

    if t_enter < 0.0 {
        if aabb.contains_point(origin) {
            let n = (delta * -1.0).normalized();
            return Some(SweepHit {
                t: 0.0,
                normal: if n.length() > 1e-6 { n } else { Vec3::Y },
            });
        }
        return None;
    }
    if t_enter > 1.0 {
        return None;
    }
    Some(SweepHit {
        t: t_enter,
        normal,
    })
}

/// 扫掠后可安全移动的位移（贴停在撞击前，留 `skin` 空隙）。
pub fn aabb_sweep_allowed(moving: Aabb3, delta: Vec3, obstacle: Aabb3, skin: f32) -> Vec3 {
    match aabb_sweep(moving, delta, obstacle) {
        None => delta,
        Some(hit) => {
            let t = (hit.t - skin).max(0.0);
            delta * t
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_hits_wall_on_x() {
        let mover = Aabb3::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let wall = Aabb3::from_min_max(Vec3::new(3.0, -1.0, -1.0), Vec3::new(4.0, 2.0, 2.0));
        let hit = aabb_sweep(mover, Vec3::new(5.0, 0.0, 0.0), wall).unwrap();
        // 中心从 0.5 扫向 5.5；扩展墙 min.x = 3-0.5 = 2.5；进入 t=(2.5-0.5)/5=0.4
        assert!((hit.t - 0.4).abs() < 1e-4, "t={}", hit.t);
        assert!(hit.normal.x < 0.0);
    }

    #[test]
    fn sweep_misses_parallel() {
        let mover = Aabb3::from_min_max(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        let wall = Aabb3::from_min_max(Vec3::new(0.0, 5.0, 0.0), Vec3::new(1.0, 6.0, 1.0));
        assert!(aabb_sweep(mover, Vec3::new(2.0, 0.0, 0.0), wall).is_none());
    }
}
