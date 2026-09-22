//! 自 `src/camera2d.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_renderer::*;
use spark_types::Vec2;

#[test]
fn roundtrip_with_zoom() {
    let cam = Camera2d::new(Vec2::new(10.0, 20.0), 2.0);
    let screen = cam.world_to_screen(Vec2::new(14.0, 25.0));
    assert!((screen.x - 8.0).abs() < 1e-5);
    assert!((screen.y - 10.0).abs() < 1e-5);
    let back = cam.screen_to_world(screen);
    assert!((back.x - 14.0).abs() < 1e-4);
    assert!((back.y - 25.0).abs() < 1e-4);
}

#[test]
fn follow_moves_origin_when_target_leaves_deadzone() {
    let mut cam = Camera2d::default();
    cam.follow_center(Vec2::new(10.0, 0.0), 0.0, 0.0, Vec2::new(2.0, 2.0), 1.0, 1.0);
    assert!((cam.origin.x - 8.0).abs() < 1e-4);
    assert!(cam.origin.y.abs() < 1e-4);
}
