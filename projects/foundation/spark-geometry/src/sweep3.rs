//! 运动 AABB 扫掠（连续碰撞原语，无游戏材质）。

use crate::{ray_aabb, Aabb3, Ray3, Vec3};

/// 扫掠命中：`toi ∈ [0, 1]` 为位移比例。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepHit {
    pub toi: f32,
    pub normal: Vec3,
}

/// 动盒 `moving` 沿 `delta` 扫掠，对静态盒 `obstacle`。
///
/// Minkowski：以动盒中心为射线原点，对扩大后的障碍 AABB 做 `ray_aabb`。
pub fn aabb_sweep(moving: Aabb3, delta: Vec3, obstacle: Aabb3) -> Option<SweepHit> {
    let ext = moving.extents();
    let expanded = Aabb3 {
        min: obstacle.min - ext,
        max: obstacle.max + ext,
    };
    let origin = moving.center();

    if delta.length() < 1e-8 {
        return if moving.intersects(obstacle) {
            Some(SweepHit {
                toi: 0.0,
                normal: separation_normal(moving, obstacle),
            })
        } else {
            None
        };
    }

    // `ray_aabb` 内部单位化方向，返回世界距离。
    let t = ray_aabb(Ray3::new(origin, delta), expanded)?;
    let len = delta.length();
    if t > len + 1e-4 {
        return None;
    }
    let toi = (t / len).clamp(0.0, 1.0);
    let at = origin + delta.normalized() * t;
    let normal = closest_face_normal(at, expanded);
    Some(SweepHit { toi, normal })
}

/// 将动盒沿 `delta` 推进到扫掠命中前（留 `skin` 世界单位余量）。
pub fn aabb_sweep_resolve(
    moving: Aabb3,
    delta: Vec3,
    obstacle: Aabb3,
    skin: f32,
) -> (Aabb3, Option<SweepHit>) {
    let ext = moving.extents();
    match aabb_sweep(moving, delta, obstacle) {
        None => {
            let c = moving.center() + delta;
            (Aabb3::from_center_extents(c, ext), None)
        }
        Some(hit) => {
            let len = delta.length().max(1e-6);
            let t = (hit.toi - skin / len).clamp(0.0, 1.0);
            let c = moving.center() + delta * t;
            (Aabb3::from_center_extents(c, ext), Some(hit))
        }
    }
}

fn separation_normal(a: Aabb3, b: Aabb3) -> Vec3 {
    let d = a.center() - b.center();
    if d.length() < 1e-8 {
        return Vec3::Y;
    }
    let ax = d.x.abs();
    let ay = d.y.abs();
    let az = d.z.abs();
    if ax >= ay && ax >= az {
        Vec3::new(d.x.signum(), 0.0, 0.0)
    } else if ay >= az {
        Vec3::new(0.0, d.y.signum(), 0.0)
    } else {
        Vec3::new(0.0, 0.0, d.z.signum())
    }
}

fn closest_face_normal(p: Vec3, box_: Aabb3) -> Vec3 {
    let candidates = [
        ((p.x - box_.min.x).abs(), Vec3::new(-1.0, 0.0, 0.0)),
        ((p.x - box_.max.x).abs(), Vec3::new(1.0, 0.0, 0.0)),
        ((p.y - box_.min.y).abs(), Vec3::new(0.0, -1.0, 0.0)),
        ((p.y - box_.max.y).abs(), Vec3::new(0.0, 1.0, 0.0)),
        ((p.z - box_.min.z).abs(), Vec3::new(0.0, 0.0, -1.0)),
        ((p.z - box_.max.z).abs(), Vec3::new(0.0, 0.0, 1.0)),
    ];
    candidates
        .into_iter()
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, n)| n)
        .unwrap_or(Vec3::Y)
}
