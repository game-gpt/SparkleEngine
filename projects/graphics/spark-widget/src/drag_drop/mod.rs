//! 拖放状态与载荷。

use std::any::{Any, TypeId};
use std::sync::Arc;

use spark_core::Vec2;

use crate::id::WidgetId;

#[derive(Clone)]
pub struct DragPayload {
    pub type_id: TypeId,
    pub data: Arc<dyn Any + Send + Sync>,
}

impl DragPayload {
    pub fn new<T: Any + Send + Sync>(value: T) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            data: Arc::new(value),
        }
    }

    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.data.downcast_ref::<T>()
    }
}

impl std::fmt::Debug for DragPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DragPayload")
            .field("type_id", &self.type_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Default)]
pub struct DragState {
    pub active: Option<WidgetId>,
    pub payload: Option<DragPayload>,
    pub origin: Vec2,
    pub position: Vec2,
    pub hover_target: Option<WidgetId>,
    /// 按下后移动超过该距离才进入拖动。
    pub threshold: f32,
    pending: Option<(WidgetId, Vec2)>,
}

impl DragState {
    pub fn begin_press(&mut self, source: WidgetId, pos: Vec2) {
        self.pending = Some((source, pos));
        self.origin = pos;
        self.position = pos;
    }

    pub fn update_move(&mut self, pos: Vec2, payload: Option<DragPayload>) -> bool {
        self.position = pos;
        if self.active.is_some() {
            return true;
        }
        let Some((source, origin)) = self.pending else {
            return false;
        };
        let dx = pos.x - origin.x;
        let dy = pos.y - origin.y;
        let threshold = if self.threshold > 0.0 {
            self.threshold
        } else {
            6.0
        };
        if dx * dx + dy * dy >= threshold * threshold {
            self.active = Some(source);
            if payload.is_some() {
                self.payload = payload;
            }
            self.pending = None;
            true
        } else {
            false
        }
    }

    pub fn end(&mut self) -> Option<(WidgetId, Option<DragPayload>, Option<WidgetId>)> {
        self.pending = None;
        let source = self.active.take()?;
        let payload = self.payload.take();
        let target = self.hover_target.take();
        Some((source, payload, target))
    }

    pub fn cancel(&mut self) {
        self.pending = None;
        self.active = None;
        self.payload = None;
        self.hover_target = None;
    }

    pub fn is_dragging(&self) -> bool {
        self.active.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
