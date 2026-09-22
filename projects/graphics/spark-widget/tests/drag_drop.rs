//! 自 `src/drag_drop/mod.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_types::Vec2;
#[test]
fn threshold_gates_drag_start() {
    let mut drag = DragState::default();
    drag.begin_press(WidgetId(1), Vec2::new(0.0, 0.0));
    assert!(!drag.update_move(Vec2::new(2.0, 0.0), None));
    assert!(!drag.is_dragging());
    assert!(drag.update_move(Vec2::new(10.0, 0.0), Some(DragPayload::new(7_u32))));
    assert!(drag.is_dragging());
    let (src, payload, _) = drag.end().unwrap();
    assert_eq!(src, WidgetId(1));
    assert_eq!(payload.unwrap().downcast_ref::<u32>(), Some(&7));
}
