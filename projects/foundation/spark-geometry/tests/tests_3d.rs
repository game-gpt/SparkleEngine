//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests_3d`。
use spark_geometry::*;

#[test]
fn ray_hits_unit_cube() {
    let ray = Ray3::new(Vec3::new(-2.0, 0.5, 0.5), Vec3::X);
    let t = ray_aabb(ray, Aabb3::from_cell(0, 0, 0)).unwrap();
    assert!((t - 2.0).abs() < 1e-4);
}

#[test]
fn voxel_dda_finds_solid() {
    let hit = ray_voxel_dda(Vec3::new(0.5, 0.5, 0.5), Vec3::X, 10.0, 64, |x, _y, _z| x == 3).unwrap();
    assert_eq!(hit.cell, [3, 0, 0]);
    assert_eq!(hit.prev, [2, 0, 0]);
}

#[test]
fn aabb_transform_translation() {
    let a = Aabb3::from_min_max(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
    let b = a.transformed(Mat4::translation(Vec3::new(10.0, 0.0, 0.0)));
    assert!((b.min.x - 10.0).abs() < 1e-5);
    assert!((b.max.x - 11.0).abs() < 1e-5);
}

#[test]
fn sweep_hits_wall() {
    let moving = Aabb3::from_center_extents(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.4, 0.4, 0.4));
    let wall = Aabb3::from_min_max(Vec3::new(2.0, 0.0, -1.0), Vec3::new(3.0, 2.0, 1.0));
    let hit = aabb_sweep(moving, Vec3::new(5.0, 0.0, 0.0), wall).unwrap();
    assert!(hit.toi > 0.0 && hit.toi < 1.0);
}

#[test]
fn mat4_orthographic_maps_center_to_origin() {
    let m = Mat4::orthographic(-10.0, 10.0, -5.0, 5.0, 0.0, 100.0);
    let p = m.transform_point(Vec3::new(0.0, 0.0, 50.0));
    assert!(p.x.abs() < 1e-4);
    assert!(p.y.abs() < 1e-4);
    // Z ∈ [0,1]：near→0，far→1，中点约 0.5
    assert!((p.z - 0.5).abs() < 1e-3);
}

#[test]
fn mat4_inverse_roundtrip_trs() {
    let m = Mat4::from_trs(Vec3::new(1.0, 2.0, 3.0), Quat::from_axis_angle(Vec3::Y, 0.7), Vec3::new(2.0, 0.5, 1.5));
    let inv = m.try_inverse().expect("invertible");
    let i = m.mul(inv);
    for idx in 0..16 {
        let expected = if idx % 5 == 0 { 1.0 } else { 0.0 };
        assert!((i.cols[idx] - expected).abs() < 1e-4, "idx {idx}: {}", i.cols[idx]);
    }
}
