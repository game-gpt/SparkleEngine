//! 局部姿态混合与骨骼遮罩。

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
