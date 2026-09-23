//! 中央场景视口在屏幕空间的矩形。

use spark_types::Rect;

use super::DockLayoutState;

/// 计算 Scene 页签正文区（不含页签条）的屏幕像素矩形。
pub fn center_viewport_rect(screen_w: f32, screen_h: f32, dock: DockLayoutState) -> Rect {
    let x = dock.hierarchy_effective() + DockLayoutState::SPLITTER_THICKNESS;
    let w = (screen_w - x - dock.inspector_effective() - DockLayoutState::SPLITTER_THICKNESS).max(0.0);
    let y = dock.main_top() + DockLayoutState::TAB_HEIGHT;
    let h = (dock.main_height(screen_h) - dock.bottom_effective() - DockLayoutState::SPLITTER_THICKNESS - DockLayoutState::TAB_HEIGHT).max(0.0);
    Rect::new(x, y, w, h)
}
