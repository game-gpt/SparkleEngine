//! 自 `src/vec_ext.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_geometry::*;

#[test]
fn normalize_and_rotate() {
    let v = Vec2::new(3.0, 4.0);
    assert!((v.length() - 5.0).abs() < 1e-5);
    let n = v.normalized();
    assert!((n.length() - 1.0).abs() < 1e-5);
    let r = Vec2::new(1.0, 0.0).rotate(std::f32::consts::FRAC_PI_2);
    assert!(r.x.abs() < 1e-5);
    assert!((r.y - 1.0).abs() < 1e-5);
}
