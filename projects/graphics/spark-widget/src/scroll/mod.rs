//! 滚动与虚拟化。
//!
//! [`ScrollState`] 挂在 `ScrollView` / `ListView` 节点上；滚轮由事件路由写入偏移，
//! [`ensure_visible`] 用于焦点或选中项滚入可视区。

use spark_types::Vec2;

use crate::{id::WidgetId, node::WidgetKind, tree::WidgetTree};

/// 允许滚动的轴向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    /// 仅纵向。
    Vertical,
    /// 仅横向。
    Horizontal,
    /// 双轴均可。
    Both,
}

/// 滚动视口的运行时状态（偏移、内容尺寸、方向）。
#[derive(Debug, Clone)]
pub struct ScrollState {
    /// 内容相对视口的滚动偏移（向右/下为正内容移动方向的相反约定：增大 = 内容上移/左移）。
    pub offset: Vec2,
    /// 可滚动内容的逻辑尺寸。
    pub content_size: Vec2,
    /// 视口逻辑尺寸（通常等于 content_rect）。
    pub viewport_size: Vec2,
    /// 允许滚动的轴向。
    pub direction: ScrollDirection,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self { offset: Vec2::ZERO, content_size: Vec2::ZERO, viewport_size: Vec2::ZERO, direction: ScrollDirection::Vertical }
    }
}

impl ScrollState {
    /// 将 `offset` 钳制到 `[0, content - viewport]`，并清零禁用轴向。
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

    /// 应用滚轮增量（正 `delta` 通常对应内容上滚 / 偏移减小约定中的「向上」）。
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
