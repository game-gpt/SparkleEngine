//! Overlay / Modal / Tooltip 层管理（占位）。

use crate::id::WidgetId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OverlayLayer {
    Base,
    Popup,
    Tooltip,
    Modal,
    Toast,
    DragPreview,
    Cursor,
}

#[derive(Debug, Clone)]
pub struct OverlayEntry {
    pub id: WidgetId,
    pub layer: OverlayLayer,
}

#[derive(Debug, Default)]
pub struct OverlayManager {
    entries: Vec<OverlayEntry>,
}

impl OverlayManager {
    pub fn push(&mut self, id: WidgetId, layer: OverlayLayer) {
        self.entries.push(OverlayEntry { id, layer });
        self.entries.sort_by_key(|e| e.layer);
    }

    pub fn remove(&mut self, id: WidgetId) {
        self.entries.retain(|e| e.id != id);
    }

    pub fn iter(&self) -> impl Iterator<Item = &OverlayEntry> {
        self.entries.iter()
    }
}
