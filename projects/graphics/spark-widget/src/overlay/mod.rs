//! Overlay / Modal / Tooltip 层管理。

use spark_core::{Rect, Vec2};

use crate::{
    id::WidgetId,
    layout::{Layout, LayoutSpec, Size},
    tree::WidgetTree,
};

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
    /// 锚点控件：定位时贴在其下方（溢出则翻到上方）。
    pub anchor: Option<WidgetId>,
    pub dismiss_on_outside: bool,
    /// 剩余存活时间（秒）。`None` 表示不自动关闭。
    pub ttl: Option<f32>,
}

#[derive(Debug, Default)]
pub struct OverlayManager {
    entries: Vec<OverlayEntry>,
}

impl OverlayManager {
    pub fn push(&mut self, id: WidgetId, layer: OverlayLayer) {
        self.push_entry(OverlayEntry { id, layer, anchor: None, dismiss_on_outside: matches!(layer, OverlayLayer::Popup), ttl: None });
    }

    pub fn push_entry(&mut self, entry: OverlayEntry) {
        self.entries.retain(|e| e.id != entry.id);
        self.entries.push(entry);
        self.entries.sort_by_key(|e| e.layer);
    }

    pub fn remove(&mut self, id: WidgetId) {
        self.entries.retain(|e| e.id != id);
    }

    pub fn remove_layer(&mut self, layer: OverlayLayer) -> Vec<WidgetId> {
        let mut removed = Vec::new();
        self.entries.retain(|e| {
            if e.layer == layer {
                removed.push(e.id);
                false
            }
            else {
                true
            }
        });
        removed
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
        self.entries.iter().rev().find(|e| e.layer == OverlayLayer::Modal).map(|e| e.id)
    }

    /// 弹出并返回最顶层条目。
    pub fn pop_top(&mut self) -> Option<OverlayEntry> {
        self.entries.pop()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 推进 TTL，返回到期应关闭的 overlay id。
    pub fn tick(&mut self, dt: f32) -> Vec<WidgetId> {
        let mut expired = Vec::new();
        for entry in &mut self.entries {
            if let Some(ttl) = entry.ttl.as_mut() {
                *ttl -= dt;
                if *ttl <= 0.0 {
                    expired.push(entry.id);
                }
            }
        }
        for id in &expired {
            self.remove(*id);
        }
        expired
    }

    /// 根据锚点把 Absolute overlay 放到屏幕内。
    pub fn position_anchored(&self, tree: &mut WidgetTree, screen: Vec2) {
        let snapshot: Vec<_> = self.entries.iter().cloned().collect();
        for entry in snapshot {
            let Some(anchor_id) = entry.anchor
            else {
                continue;
            };
            let Some(anchor) = tree.node(anchor_id).map(|n| n.computed.rect)
            else {
                continue;
            };
            let desired = tree.node(entry.id).map(|n| n.computed.desired).unwrap_or(crate::layout::Size2::new(160.0, 40.0));
            let w = desired.width.max(1.0);
            let h = desired.height.max(1.0);
            let mut x = anchor.x;
            let mut y = anchor.y + anchor.h + 4.0;
            if y + h > screen.y {
                y = (anchor.y - h - 4.0).max(0.0);
            }
            if x + w > screen.x {
                x = (screen.x - w).max(0.0);
            }
            x = x.max(0.0);
            if let Some(node) = tree.node_mut(entry.id) {
                node.layout = LayoutSpec {
                    kind: Layout::Absolute,
                    width: Size::Px(w),
                    height: Size::Px(h),
                    offset_x: x,
                    offset_y: y,
                    ..LayoutSpec::default()
                };
                node.computed.rect = Rect::new(x, y, w, h);
                node.computed.content_rect = node.computed.rect;
            }
        }
    }
}
