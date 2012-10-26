//! 虚拟列表：只构建可见行。

use std::hash::Hash;
use std::ops::Range;

use spark_core::Rect;

use crate::layout::{Layout, LayoutCursor};
use crate::response::Response;
use crate::ui::Ui;

impl Ui<'_> {
    /// 固定行高的虚拟列表。`f` 收到可见行下标范围（半开）。
    pub fn virtual_list<R>(
        &mut self,
        salt: impl Hash,
        item_count: usize,
        row_height: f32,
        viewport_height: f32,
        f: impl FnOnce(&mut Self, Range<usize>) -> R,
    ) -> (Response, R) {
        let row_height = row_height.max(1.0);
        let id = self.id_from(("virtual_list", salt));
        let total_h = item_count as f32 * row_height;
        let viewport = self.allocate(viewport_height.max(row_height), None);

        let (mx, my) = self.input.mouse_pos();
        let hovering = viewport.contains(spark_core::Vec2::new(mx, my));
        if hovering {
            let mem = self.state.memory_mut(id);
            mem.scroll.y -= self.input.wheel() * row_height;
        }

        let max_scroll = (total_h - viewport.h).max(0.0);
        {
            let mem = self.state.memory_mut(id);
            mem.scroll.y = mem.scroll.y.clamp(0.0, max_scroll);
            mem.last_rect = Some(viewport);
        }
        let scroll_y = self.state.memory(id).map(|m| m.scroll.y).unwrap_or(0.0);

        let first = (scroll_y / row_height).floor().max(0.0) as usize;
        let visible = (viewport.h / row_height).ceil() as usize + 1;
        let last = (first + visible).min(item_count);
        let range = first..last;

        self.draw.fill_rect(viewport, self.theme.colors.panel);
        self.push_clip(viewport);

        let y0 = viewport.y - (scroll_y - first as f32 * row_height);
        let content = Rect::new(viewport.x, y0, viewport.w, (last - first) as f32 * row_height);
        self.layouts.push(LayoutCursor::new(
            content,
            Layout::vertical().gap(0.0),
        ));

        // 把行高塞进布局：调用方用 allocate(row_height) 画每一行。
        let out = self.scope(("virtual_body", id.raw()), |ui| f(ui, range.clone()));

        self.layouts.pop();
        self.pop_clip();

        let mut response = self.interact(id, viewport, false);
        response.hovered = hovering || response.hovered;
        (response, out)
    }
}
