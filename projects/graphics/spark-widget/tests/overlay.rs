//! 自 `src/overlay/mod.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

#[test]
fn modal_sorts_above_popup() {
    let mut overlays = OverlayManager::default();
    overlays.push(WidgetId(1), OverlayLayer::Popup);
    overlays.push(WidgetId(2), OverlayLayer::Modal);
    overlays.push(WidgetId(3), OverlayLayer::Tooltip);
    assert_eq!(overlays.top().map(|e| e.id), Some(WidgetId(2)));
    assert_eq!(overlays.top_modal(), Some(WidgetId(2)));
}
