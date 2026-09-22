//! 自 `src/periodic.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_geometry::*;

#[test]
fn floor_mod_wraps_negative() {
    assert!((floor_mod(-1.0, 10.0) - 9.0).abs() < 1e-5);
    assert!((floor_mod(10.0, 10.0)).abs() < 1e-5);
    assert!((floor_mod(25.5, 10.0) - 5.5).abs() < 1e-5);
}

#[test]
fn shortest_delta_across_seam() {
    let d = shortest_delta_1d(1.0, 9.0, 10.0);
    assert!((d - (-2.0)).abs() < 1e-5);
    let d2 = shortest_delta_1d(9.0, 1.0, 10.0);
    assert!((d2 - 2.0).abs() < 1e-5);
}

#[test]
fn xz_normalize() {
    let p = normalize_position(Vec3::new(12.0, 3.0, -1.0), PeriodicAxes::XZ, PeriodSize::xz(10.0, 8.0));
    assert!((p.x - 2.0).abs() < 1e-5);
    assert!((p.y - 3.0).abs() < 1e-5);
    assert!((p.z - 7.0).abs() < 1e-5);
}
