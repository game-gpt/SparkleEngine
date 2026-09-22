//! 自 `src/scroll/mod.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_types::Vec2;
#[test]
fn clamp_keeps_offset_in_range() {
    let mut scroll = ScrollState {
        offset: Vec2::new(0.0, 999.0),
        content_size: Vec2::new(100.0, 400.0),
        viewport_size: Vec2::new(100.0, 100.0),
        direction: ScrollDirection::Vertical,
    };
    scroll.clamp_offset();
    assert!((scroll.offset.y - 300.0).abs() < 0.01);
}
