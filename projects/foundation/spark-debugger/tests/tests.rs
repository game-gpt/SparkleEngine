//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_debugger::*;

#[test]
fn debug_draw_respects_enabled_flag() {
    let mut d = DebugDraw::new();
    d.rect_filled(Rect::new(0.0, 0.0, 10.0, 10.0), Color::rgb(1.0, 0.0, 0.0));
    assert_eq!(d.prims().len(), 1);
    d.set_enabled(false);
    d.clear();
    d.line(Vec2::ZERO, Vec2::new(1.0, 1.0), Color::rgb(1.0, 1.0, 1.0), 1.0);
    assert!(d.prims().is_empty());
}
