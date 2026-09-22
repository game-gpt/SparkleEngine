//! 自 `src/trs.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_geometry::*;

#[test]
fn identity_mat_is_identity() {
    let m = Trs::IDENTITY.to_mat4();
    for i in 0..16 {
        let expected = if i % 5 == 0 { 1.0 } else { 0.0 };
        assert!((m.cols[i] - expected).abs() < 1e-6);
    }
}

#[test]
fn translation_only() {
    let m = Trs::from_translation(Vec3::new(1.0, 2.0, 3.0)).to_mat4();
    let p = m.transform_point(Vec3::ZERO);
    assert!((p.x - 1.0).abs() < 1e-5);
    assert!((p.y - 2.0).abs() < 1e-5);
    assert!((p.z - 3.0).abs() < 1e-5);
}
