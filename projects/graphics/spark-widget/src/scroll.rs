//! 滚动区域：持久偏移、滚轮、裁剪与简易滚动条。

use std::hash::Hash;

use spark_core::{Rect, Vec2};
use spark_input::MouseBtn;

use crate::layout::{Layout, LayoutCursor};
use crate::response::Response;
use crate::ui::Ui;

impl Ui<'_> {
    /// 垂直滚动区。内容在裁剪视口内绘制，滚轮在悬停时生效。
    pub fn scroll_area<R>(
        &mut self,
        salt: impl Hash,
        height: f32,
        f: impl FnOnce(&mut Self) -> R,
    ) -> (Response, R) {
        let id = self.id_from(("scroll", salt));
        let bar_w = 10.0;
        let viewport = self.allocate(height.max(1.0), None);
        let content_w = (viewport.w - bar_w - 2.0).max(1.0);
        let view = Rect::new(viewport.x, viewport.y, content_w, viewport.h);

        let (mx, my) = self.input.mouse_pos();
        let pointer = Vec2::new(mx, my);
        let hovering = view.contains(pointer) || viewport.contains(pointer);

        {
            let mem = self.state.memory_mut(id);
            if hovering {
                mem.scroll.y -= self.input.wheel() * 40.0;
            }
        }

        self.draw.fill_rect(viewport, self.theme.colors.panel);
        self.push_clip(view);

        let scroll_y = self.state.memory(id).map(|m| m.scroll.y).unwrap_or(0.0);
        let content_origin = Rect::new(view.x, view.y - scroll_y, view.w, 1_000_000.0);
        self.layouts
            .push(LayoutCursor::new(content_origin, Layout::vertical().gap(self.theme.spacing.sm)));

        let out = self.scope(("scroll_body", id.raw()), f);

        let used = self
            .layouts
            .last()
            .map(|c| (c.cursor.y - content_origin.y).max(0.0))
            .unwrap_or(0.0);
        self.layouts.pop();
        self.pop_clip();

        let max_scroll = (used - view.h).max(0.0);
        {
            let mem = self.state.memory_mut(id);
            mem.scroll.y = mem.scroll.y.clamp(0.0, max_scroll);
            mem.last_rect = Some(viewport);
            mem.f32_value = used;
        }
        let scroll_y = self.state.memory(id).map(|m| m.scroll.y).unwrap_or(0.0);

        // 滚动条
        let bar = Rect::new(view.x + view.w + 2.0, view.y, bar_w, view.h);
        self.draw.fill_rect(bar, self.theme.colors.track);
        if max_scroll > 1.0 {
            let thumb_h = (view.h * (view.h / used)).clamp(16.0, view.h);
            let t = scroll_y / max_scroll;
            let thumb_y = view.y + t * (view.h - thumb_h);
            let thumb = Rect::new(bar.x + 1.0, thumb_y, bar.w - 2.0, thumb_h);
            let thumb_hover = thumb.contains(pointer) || bar.contains(pointer);
            if thumb_hover && self.input.mouse_pressed(MouseBtn::Left) {
                self.capture(id);
            }
            if self.state.captured == Some(id) && self.input.mouse_down(MouseBtn::Left) {
                let rel = ((my - view.y) / view.h).clamp(0.0, 1.0);
                self.state.memory_mut(id).scroll.y = rel * max_scroll;
            }
            self.draw.fill_rect(thumb, self.theme.colors.knob);
        }

        let mut response = self.interact(id, viewport, false);
        response.hovered = hovering || response.hovered;
        (response, out)
    }
}
