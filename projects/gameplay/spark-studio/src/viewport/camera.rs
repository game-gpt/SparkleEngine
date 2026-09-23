//! 场景视口世界 / 屏幕坐标换算。

use spark_types::Rect;

use crate::state::ViewportState;

/// 视口中心（屏幕像素，含平移）。
pub fn viewport_center(rect: Rect, vp: &ViewportState) -> (f32, f32) {
    (rect.x + rect.w * 0.5 + vp.pan_x, rect.y + rect.h * 0.5 + vp.pan_y)
}

/// 世界坐标 → 屏幕像素（Y 轴向上）。
pub fn world_to_screen(wx: f32, wy: f32, rect: Rect, vp: &ViewportState) -> (f32, f32) {
    let (cx, cy) = viewport_center(rect, vp);
    (cx + wx * vp.zoom, cy - wy * vp.zoom)
}

/// 屏幕像素 → 世界坐标（Y 轴向上）。
pub fn screen_to_world(sx: f32, sy: f32, rect: Rect, vp: &ViewportState) -> (f32, f32) {
    let (cx, cy) = viewport_center(rect, vp);
    ((sx - cx) / vp.zoom, (cy - sy) / vp.zoom)
}
