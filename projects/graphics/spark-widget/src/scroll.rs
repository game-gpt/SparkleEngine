//! 滚动区域：持久偏移、滚轮、裁剪、滚动条与滚入可视。

use std::hash::Hash;

use spark_core::{Rect, Vec2};
use spark_input::MouseBtn;

use crate::id::WidgetId;
use crate::layout::{Layout, LayoutCursor};
use crate::response::Response;
use crate::state::{FocusSource, ScrollAreaFrame, UiState};
use crate::ui::Ui;

impl Ui<'_> {
    /// 请求让控件在所属 [`Self::scroll_area`] 内可见（本帧末或下帧生效）。
    pub fn scroll_into_view(&mut self, id: WidgetId) {
        self.state.request_scroll_into_view(id);
    }

    /// 垂直滚动区。内容在裁剪视口内绘制，滚轮在悬停时生效。
    /// 焦点落在区内外且被键盘导航时，会自动滚入可视。
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

        self.state.scroll_areas.push(ScrollAreaFrame {
            id,
            view,
            content_top: content_origin.y,
            used,
            max_scroll,
        });

        let scroll_y = self.state.memory(id).map(|m| m.scroll.y).unwrap_or(0.0);

        // 滚动条
        let bar = Rect::new(view.x + view.w + 2.0, view.y, bar_w, view.h);
        self.draw.fill_rect(bar, self.theme.colors.track);
        if max_scroll > 1.0 {
            let thumb_h = (view.h * (view.h / used)).clamp(16.0, view.h);
            let t = if max_scroll > 0.0 {
                scroll_y / max_scroll
            } else {
                0.0
            };
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

    /// 水平滚动区。滚轮在悬停时左右平移。
    pub fn scroll_area_x<R>(
        &mut self,
        salt: impl Hash,
        height: f32,
        f: impl FnOnce(&mut Self) -> R,
    ) -> (Response, R) {
        let id = self.id_from(("scroll_x", salt));
        let bar_h = 10.0;
        let viewport = self.allocate(height.max(bar_h + 8.0), None);
        let content_h = (viewport.h - bar_h - 2.0).max(1.0);
        let view = Rect::new(viewport.x, viewport.y, viewport.w, content_h);

        let (mx, my) = self.input.mouse_pos();
        let pointer = Vec2::new(mx, my);
        let hovering = view.contains(pointer) || viewport.contains(pointer);

        {
            let mem = self.state.memory_mut(id);
            if hovering {
                mem.scroll.x -= self.input.wheel() * 40.0;
            }
        }

        self.draw.fill_rect(viewport, self.theme.colors.panel);
        self.push_clip(view);

        let scroll_x = self.state.memory(id).map(|m| m.scroll.x).unwrap_or(0.0);
        let content_origin = Rect::new(view.x - scroll_x, view.y, 1_000_000.0, view.h);
        self.layouts.push(LayoutCursor::new(
            content_origin,
            Layout::horizontal().gap(self.theme.spacing.sm),
        ));

        let out = self.scope(("scroll_x_body", id.raw()), f);

        let used = self
            .layouts
            .last()
            .map(|c| (c.cursor.x - content_origin.x).max(0.0))
            .unwrap_or(0.0);
        self.layouts.pop();
        self.pop_clip();

        let max_scroll = (used - view.w).max(0.0);
        {
            let mem = self.state.memory_mut(id);
            mem.scroll.x = mem.scroll.x.clamp(0.0, max_scroll);
            mem.last_rect = Some(viewport);
            mem.f32_value = used;
        }
        let scroll_x = self.state.memory(id).map(|m| m.scroll.x).unwrap_or(0.0);

        let bar = Rect::new(view.x, view.y + view.h + 2.0, view.w, bar_h);
        self.draw.fill_rect(bar, self.theme.colors.track);
        if max_scroll > 1.0 {
            let thumb_w = (view.w * (view.w / used)).clamp(16.0, view.w);
            let t = if max_scroll > 0.0 {
                scroll_x / max_scroll
            } else {
                0.0
            };
            let thumb_x = view.x + t * (view.w - thumb_w);
            let thumb = Rect::new(thumb_x, bar.y + 1.0, thumb_w, bar.h - 2.0);
            let thumb_hover = thumb.contains(pointer) || bar.contains(pointer);
            if thumb_hover && self.input.mouse_pressed(MouseBtn::Left) {
                self.capture(id);
            }
            if self.state.captured == Some(id) && self.input.mouse_down(MouseBtn::Left) {
                let rel = ((mx - view.x) / view.w).clamp(0.0, 1.0);
                self.state.memory_mut(id).scroll.x = rel * max_scroll;
            }
            self.draw.fill_rect(thumb, self.theme.colors.knob);
        }

        let mut response = self.interact(id, viewport, false);
        response.hovered = hovering || response.hovered;
        (response, out)
    }
}

impl UiState {
    /// 帧末：按焦点 / 显式请求修正各滚动区偏移。
    pub(crate) fn apply_scroll_into_view_all(&mut self) {
        let mut targets: Vec<WidgetId> = self.scroll_into_view.clone();
        if self.focus_source == FocusSource::Keyboard {
            if let Some(fid) = self.focused {
                if !targets.contains(&fid) {
                    targets.push(fid);
                }
            }
        }
        if targets.is_empty() || self.scroll_areas.is_empty() {
            return;
        }

        let areas = self.scroll_areas.clone();
        let mut consumed = Vec::new();
        for area in areas {
            let content_bottom = area.content_top + area.used;
            let mut scroll_y = self.memory(area.id).map(|m| m.scroll.y).unwrap_or(0.0);
            let mut changed = false;
            for tid in &targets {
                let Some(frect) = self.focus_rects.get(tid).copied() else {
                    continue;
                };
                let overlaps_x =
                    frect.x + frect.w > area.view.x && frect.x < area.view.x + area.view.w;
                let in_content = frect.y + frect.h > area.content_top
                    && frect.y < content_bottom
                    && overlaps_x;
                if !in_content {
                    continue;
                }
                if frect.y < area.view.y {
                    scroll_y -= area.view.y - frect.y;
                    changed = true;
                } else if frect.y + frect.h > area.view.y + area.view.h {
                    scroll_y += (frect.y + frect.h) - (area.view.y + area.view.h);
                    changed = true;
                }
                consumed.push(*tid);
            }
            if changed {
                self.memory_mut(area.id).scroll.y = scroll_y.clamp(0.0, area.max_scroll);
            }
        }
        self.scroll_into_view
            .retain(|id| !consumed.contains(id));
    }
}
