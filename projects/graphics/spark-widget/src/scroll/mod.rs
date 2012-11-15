//! 滚动与虚拟化（占位）。

use spark_core::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Vertical,
    Horizontal,
    Both,
}

#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    pub offset: Vec2,
    pub content_size: Vec2,
    pub viewport_size: Vec2,
}

impl ScrollState {
    pub fn clamp_offset(&mut self) {
        let max_x = (self.content_size.x - self.viewport_size.x).max(0.0);
        let max_y = (self.content_size.y - self.viewport_size.y).max(0.0);
        self.offset.x = self.offset.x.clamp(0.0, max_x);
        self.offset.y = self.offset.y.clamp(0.0, max_y);
    }
}
