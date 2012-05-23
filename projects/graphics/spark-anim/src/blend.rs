//! 局部姿态混合与骨骼遮罩。

use spark_geometry::{Quat, Trs, Vec3};

use crate::pose::LocalPose;

/// 两套局部姿态按统一权重混合：`out = lerp(a, b, weight)`。
pub fn blend_local_poses(a: &LocalPose, b: &LocalPose, weight: f32) -> LocalPose {
    let w = weight.clamp(0.0, 1.0);
    let n = a.locals.len().min(b.locals.len());
    let mut locals = Vec::with_capacity(n);
    for i in 0..n {
        locals.push(a.locals[i].lerp(b.locals[i], w));
    }
    LocalPose { locals }
}

/// 按关节权重混合：`weights[i]` 为取自 `b` 的比例（上半身/下半身遮罩）。
///
/// `weights` 短于关节数时，缺省关节权重为 0（完全保留 `a`）。
pub fn blend_masked(a: &LocalPose, b: &LocalPose, weights: &[f32]) -> LocalPose {
    let n = a.locals.len().min(b.locals.len());
    let mut locals = Vec::with_capacity(n);
    for i in 0..n {
        let w = weights.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
        locals.push(if w <= 0.0 {
            a.locals[i]
        } else if w >= 1.0 {
            b.locals[i]
        } else {
            a.locals[i].lerp(b.locals[i], w)
        });
    }
    // 保留 a 中多出的关节（若有）
    for i in n..a.locals.len() {
        locals.push(a.locals[i]);
    }
    LocalPose { locals }
}

/// 加性混合：把 `layer` 相对 `rest` 的增量叠到 `base` 上。
///
/// ```text
/// Δt = layer.t - rest.t
/// Δr = rest.r⁻¹ * layer.r
/// Δs = layer.s - rest.s
/// out.t = base.t + Δt * w
/// out.r = base.r * nlerp(I, Δr, w)
/// out.s = base.s + Δs * w
/// ```
///
/// 用于喷气姿态、受击等 additive 层；`weight` 钳到 `[0, 1]`。
pub fn add_local_poses(
    base: &LocalPose,
    layer: &LocalPose,
    rest: &LocalPose,
    weight: f32,
) -> LocalPose {
    let w = weight.clamp(0.0, 1.0);
    let n = base
        .locals
        .len()
        .min(layer.locals.len())
        .min(rest.locals.len());
    let mut locals = Vec::with_capacity(base.locals.len());
    for i in 0..n {
        let b = base.locals[i];
        let l = layer.locals[i];
        let r = rest.locals[i];
        if w <= 0.0 {
            locals.push(b);
            continue;
        }
        let dt = (l.translation - r.translation) * w;
        let ds = (l.scale - r.scale) * w;
        // rest⁻¹ * layer（单位四元数用共轭当逆）
        let delta_r = r.rotation.conjugate().mul(l.rotation).normalized();
        let add_r = Quat::IDENTITY.nlerp(delta_r, w);
        locals.push(Trs {
            translation: b.translation + dt,
            rotation: b.rotation.mul(add_r).normalized(),
            scale: Vec3::new(b.scale.x + ds.x, b.scale.y + ds.y, b.scale.z + ds.z),
        });
    }
    for i in n..base.locals.len() {
        locals.push(base.locals[i]);
    }
    LocalPose { locals }
}
