//! 自 `src/quat.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_geometry::*;

#[test]
fn identity_leaves_vector() {
    let v = Vec3::new(1.0, 2.0, 3.0);
    let r = Quat::IDENTITY.rotate_vec3(v);
    assert!((r.x - 1.0).abs() < 1e-5);
    assert!((r.y - 2.0).abs() < 1e-5);
    assert!((r.z - 3.0).abs() < 1e-5);
}

#[test]
fn yaw_90_maps_x_to_neg_z() {
    let q = Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2);
    let r = q.rotate_vec3(Vec3::X);
    assert!((r.x - 0.0).abs() < 1e-4);
    assert!((r.y - 0.0).abs() < 1e-4);
    assert!((r.z + 1.0).abs() < 1e-4);
}

#[test]
fn slerp_midpoint_unit() {
    let a = Quat::IDENTITY;
    let b = Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2);
    let m = a.slerp(b, 0.5);
    assert!((m.length() - 1.0).abs() < 1e-5);
}

#[test]
fn rotation_between_maps_axis() {
    let q = Quat::rotation_between(Vec3::X, Vec3::Y);
    let v = q.rotate_vec3(Vec3::X);
    assert!(v.x.abs() < 1e-4);
    assert!((v.y - 1.0).abs() < 1e-4);
}
