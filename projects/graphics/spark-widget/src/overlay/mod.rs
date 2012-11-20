//! Overlay / Modal / Tooltip 层管理。

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
        self.entries.retain(|e| e.id != id);
        self.entries.push(OverlayEntry { id, layer });
        self.entries.sort_by_key(|e| e.layer);
    }

    pub fn remove(&mut self, id: WidgetId) {
        self.entries.retain(|e| e.id != id);
    }

    pub fn iter(&self) -> impl Iterator<Item = &OverlayEntry> {
        self.entries.iter()
    }

    pub fn contains(&self, id: WidgetId) -> bool {
        self.entries.iter().any(|e| e.id == id)
    }

    /// 最顶层（排序后最后一个）条目。
    pub fn top(&self) -> Option<&OverlayEntry> {
        self.entries.last()
    }

    pub fn top_modal(&self) -> Option<WidgetId> {
        self.entries
            .iter()
            .rev()
            .find(|e| e.layer == OverlayLayer::Modal)
            .map(|e| e.id)
    }

    /// 弹出并返回最顶层条目。
    pub fn pop_top(&mut self) -> Option<OverlayEntry> {
        self.entries.pop()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modal_sorts_above_popup() {
        let mut overlays = OverlayManager::default();
        overlays.push(WidgetId(1), OverlayLayer::Popup);
        overlays.push(WidgetId(2), OverlayLayer::Modal);
        overlays.push(WidgetId(3), OverlayLayer::Tooltip);
        assert_eq!(overlays.top().map(|e| e.id), Some(WidgetId(2)));
        assert_eq!(overlays.top_modal(), Some(WidgetId(2)));
    }
}
