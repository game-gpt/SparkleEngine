//! 拖放（占位）。

use std::any::{Any, TypeId};
use std::sync::Arc;

use crate::id::WidgetId;

#[derive(Clone)]
pub struct DragPayload {
    pub type_id: TypeId,
    pub data: Arc<dyn Any + Send + Sync>,
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
}
