//! 滚动与虚拟化。

use spark_core::Vec2;

use crate::{id::WidgetId, node::WidgetKind, tree::WidgetTree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Vertical,
    Horizontal,
    Both,
}

#[derive(Debug, Clone)]
pub struct ScrollState {
    pub offset: Vec2,
    pub content_size: Vec2,
    pub viewport_size: Vec2,
    pub direction: ScrollDirection,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self { offset: Vec2::ZERO, content_size: Vec2::ZERO, viewport_size: Vec2::ZERO, direction: ScrollDirection::Vertical }
    }
}

impl ScrollState {
    pub fn clamp_offset(&mut self) {
        let max_x = (self.content_size.x - self.viewport_size.x).max(0.0);
        let max_y = (self.content_size.y - self.viewport_size.y).max(0.0);
        match self.direction {
            ScrollDirection::Vertical => {
                self.offset.x = 0.0;
                self.offset.y = self.offset.y.clamp(0.0, max_y);
            }
            ScrollDirection::Horizontal => {
                self.offset.y = 0.0;
                self.offset.x = self.offset.x.clamp(0.0, max_x);
            }
            ScrollDirection::Both => {
                self.offset.x = self.offset.x.clamp(0.0, max_x);
                self.offset.y = self.offset.y.clamp(0.0, max_y);
            }
        }
    }

    pub fn apply_wheel(&mut self, delta: f32) {
        match self.direction {
            ScrollDirection::Horizontal => self.offset.x -= delta,
            ScrollDirection::Vertical | ScrollDirection::Both => self.offset.y -= delta,
        }
        self.clamp_offset();
    }
}

/// 从命中节点向上找最近的 `ScrollView`。
pub fn find_scroll_ancestor(tree: &WidgetTree, mut id: WidgetId) -> Option<WidgetId> {
    loop {
        let node = tree.node(id)?;
        if matches!(node.kind, WidgetKind::ScrollView | WidgetKind::ListView) {
            return Some(id);
        }
        id = node.parent?;
    }
}

/// 调整最近 `ScrollView` 的偏移，使 `target` 落在可视区内。
pub fn ensure_visible(tree: &mut WidgetTree, target: WidgetId) {
    let Some(scroll_id) = find_scroll_ancestor(tree, target)
    else {
        return;
    };
    let (target_rect, clip, offset, content_size, viewport_size) = {
        let Some(target_node) = tree.node(target)
        else {
            return;
        };
        let Some(scroll_node) = tree.node(scroll_id)
        else {
            return;
        };
        let clip = scroll_node.computed.clip_rect.unwrap_or(scroll_node.computed.content_rect);
        (target_node.computed.rect, clip, scroll_node.scroll.offset, scroll_node.scroll.content_size, scroll_node.scroll.viewport_size)
    };

    let mut new_offset = offset;
    if target_rect.y < clip.y {
        new_offset.y -= clip.y - target_rect.y;
    }
    else if target_rect.y + target_rect.h > clip.y + clip.h {
        new_offset.y += (target_rect.y + target_rect.h) - (clip.y + clip.h);
    }
    if target_rect.x < clip.x {
        new_offset.x -= clip.x - target_rect.x;
    }
    else if target_rect.x + target_rect.w > clip.x + clip.w {
        new_offset.x += (target_rect.x + target_rect.w) - (clip.x + clip.w);
    }

    if let Some(node) = tree.node_mut(scroll_id) {
        node.scroll.offset = new_offset;
        node.scroll.content_size = content_size;
        node.scroll.viewport_size = viewport_size;
        node.scroll.clamp_offset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_keeps_offset_in_range() {
        let mut scroll = ScrollState {
            offset: Vec2::new(0.0, 999.0),
            content_size: Vec2::new(100.0, 400.0),
            viewport_size: Vec2::new(100.0, 100.0),
            direction: ScrollDirection::Vertical,
        };
        scroll.clamp_offset();
        assert!((scroll.offset.y - 300.0).abs() < 0.01);
    }
}
