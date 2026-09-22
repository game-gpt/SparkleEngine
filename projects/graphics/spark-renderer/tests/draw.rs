//! 自 `src/draw.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_core::{Color, Rect, Vec2};
use spark_renderer::*;

#[test]
fn world_quads_follow_camera_and_hud_does_not() {
    let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
    draw.set_camera(Camera2d::new(Vec2::new(8.0, 0.0), 2.0));
    draw.fill_rect(Rect::new(10.0, 0.0, 4.0, 2.0), Color::rgb(1.0, 0.0, 0.0));
    draw.begin_hud();
    draw.fill_rect(Rect::new(10.0, 0.0, 4.0, 2.0), Color::rgb(0.0, 1.0, 0.0));
    assert!((draw.quads[0].rect.x - 4.0).abs() < 1e-5);
    assert!((draw.quads[0].rect.w - 8.0).abs() < 1e-5);
    assert!((draw.hud_quads[0].rect.x - 10.0).abs() < 1e-5);
    assert!((draw.hud_quads[0].rect.w - 4.0).abs() < 1e-5);
}

#[test]
fn clip_culls_and_intersects_quads() {
    let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
    draw.begin_hud();
    draw.push_clip(Rect::new(10.0, 10.0, 20.0, 20.0));
    draw.fill_rect(Rect::new(0.0, 0.0, 5.0, 5.0), Color::rgb(1.0, 0.0, 0.0));
    assert!(draw.hud_quads.is_empty());
    draw.fill_rect(Rect::new(15.0, 15.0, 20.0, 20.0), Color::rgb(0.0, 1.0, 0.0));
    assert_eq!(draw.hud_quads.len(), 1);
    assert!((draw.hud_quads[0].rect.w - 15.0).abs() < 1e-4);
    draw.pop_clip();
}
