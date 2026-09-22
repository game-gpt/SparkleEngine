//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_renderer::*;

use spark_core::Color;
use std::sync::Arc;

#[test]
fn retain_visible_drops_far_mesh() {
    let cam = Camera3d { eye: Vec3::new(0.0, 0.0, 5.0), yaw: 0.0, pitch: 0.0, fov_y_rad: 70f32.to_radians(), near: 0.1, far: 100.0 };
    let mut list = DrawList3d::new(Color::rgb(0.0, 0.0, 0.0), cam.view_proj(16.0 / 9.0));
    let verts: Arc<[MeshVertex]> = Arc::from(vec![MeshVertex::new(0.0, 0.0, 0.0, Color::rgb(1.0, 1.0, 1.0))]);
    list.mesh_culled(
        Mat4::translation(Vec3::new(0.0, 0.0, 0.0)),
        Arc::clone(&verts),
        Aabb3::from_min_max(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5)),
    );
    list.mesh_culled(
        Mat4::translation(Vec3::new(0.0, 0.0, -500.0)),
        verts,
        Aabb3::from_min_max(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5)),
    );
    assert_eq!(list.meshes.len(), 2);
    list.retain_visible(CullParams::new(cam.eye).with_max_distance(50.0));
    assert_eq!(list.meshes.len(), 1);
}

#[test]
fn create_texture_queues_upload() {
    let mut list = DrawList3d::new(Color::rgb(0.0, 0.0, 0.0), Mat4::IDENTITY);
    let id = list.create_texture(1, 1, vec![255, 0, 0, 255]).unwrap();
    assert!(id.0 >= 1);
    assert_eq!(list.texture_uploads.len(), 1);
}

#[test]
fn skinned_mesh_truncates_palette() {
    let mut list = DrawList3d::new(Color::rgb(0.0, 0.0, 0.0), Mat4::IDENTITY);
    let vert = SkinnedVertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0], Color::rgb(1.0, 1.0, 1.0), [0, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]);
    let palette: Vec<Mat4> = (0..MAX_SKIN_JOINTS + 8).map(|_| Mat4::IDENTITY).collect();
    list.skinned_mesh(Mat4::IDENTITY, Arc::from(vec![vert]), Arc::from(palette), None);
    assert_eq!(list.skinned_meshes.len(), 1);
    assert_eq!(list.skinned_meshes[0].joint_palette.len(), MAX_SKIN_JOINTS);
}
