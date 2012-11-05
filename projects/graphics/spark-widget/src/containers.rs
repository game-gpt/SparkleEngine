//! 分割面板与通用物品槽。

use spark_core::{Color, Rect};
use spark_input::MouseBtn;
use spark_renderer::TextureId;

use crate::accessibility::{AccessNode, Role};
use crate::layout::{Layout, LayoutCursor};
use crate::response::Response;
use crate::ui::Ui;

impl Ui<'_> {
    /// 水平分割：左侧宽度比例 `ratio`（0..1），中间可拖动分隔条。
    /// `ratio` 由调用方跨帧持有。
    pub fn split_pane(
        &mut self,
        salt: impl std::hash::Hash,
        ratio: &mut f32,
        height: f32,
        mut left: impl FnMut(&mut Self),
        mut right: impl FnMut(&mut Self),
    ) -> Response {
        let id = self.id_from(("split", salt));
        let height = height.max(40.0);
        let bounds = self.allocate(height, None);
        let gutter = 6.0;
        let mut t = ratio.clamp(0.15, 0.85);

        // 先用旧分隔区做命中，拖动时即时更新比例。
        let probe_left_w = ((bounds.w - gutter) * t).max(24.0);
        let probe_gutter = Rect::new(bounds.x + probe_left_w, bounds.y, gutter, bounds.h);
        self.state.register_focusable(id, probe_gutter);
        let mut response = self.interact(id, probe_gutter, true);
        if response.hovered && self.input.mouse_pressed(MouseBtn::Left) {
            self.capture(id);
        }
        if self.state.captured == Some(id) && self.input.mouse_down(MouseBtn::Left) {
            let (mx, _) = self.input.mouse_pos();
            let span = (bounds.w - gutter).max(1.0);
            t = ((mx - bounds.x) / span).clamp(0.15, 0.85);
            if (*ratio - t).abs() > 1e-4 {
                *ratio = t;
                response.changed = true;
            }
            response.active = true;
        } else {
            *ratio = t;
        }

        let left_w = ((bounds.w - gutter) * *ratio).max(24.0);
        let left_rect = Rect::new(bounds.x, bounds.y, left_w, bounds.h);
        let gutter_rect = Rect::new(bounds.x + left_w, bounds.y, gutter, bounds.h);
        let right_rect = Rect::new(
            gutter_rect.x + gutter,
            bounds.y,
            (bounds.w - left_w - gutter).max(24.0),
            bounds.h,
        );

        self.draw.fill_rect(bounds, self.theme.colors.panel);
        self.draw.fill_rect(
            gutter_rect,
            if response.hovered || response.active {
                self.theme.colors.primary_hover
            } else {
                self.theme.colors.track
            },
        );

        self.layouts
            .push(LayoutCursor::new(left_rect, Layout::vertical().gap(self.theme.spacing.sm)));
        self.scope(("split_left", id.raw()), |ui| left(ui));
        self.layouts.pop();

        self.layouts
            .push(LayoutCursor::new(right_rect, Layout::vertical().gap(self.theme.spacing.sm)));
        self.scope(("split_right", id.raw()), |ui| right(ui));
        self.layouts.pop();

        self.access(
            AccessNode::new(id, Role::Separator)
                .label("split")
                .value(format!("{:.2}", *ratio))
                .rect(gutter_rect)
                .focusable(true)
                .focused(response.focused),
        );
        response.rect = bounds;
        response
    }

    /// 通用槽位：固定方格，可显示纹理与数量角标。游戏语义由调用方解释。
    pub fn slot(
        &mut self,
        salt: impl std::hash::Hash,
        size: f32,
        icon: Option<(TextureId, Rect)>,
        count: Option<u32>,
        selected: bool,
    ) -> Response {
        let size = size.max(16.0);
        let rect = self.allocate(size, Some(size));
        let id = self.id_from(("slot", salt));
        self.state.register_focusable(id, rect);
        let mut response = self.interact(id, rect, true);
        if self.keyboard_activate(id) {
            response.clicked = true;
        }

        let border = if selected || response.focused {
            self.theme.colors.focus_ring
        } else if response.hovered {
            self.theme.colors.primary_hover
        } else {
            self.theme.colors.border
        };
        self.draw.fill_rect(
            Rect::new(rect.x - 1.0, rect.y - 1.0, rect.w + 2.0, rect.h + 2.0),
            border,
        );
        self.draw
            .fill_rect(rect, Color::rgb(0.08, 0.10, 0.16));
        if let Some((tex, uv)) = icon {
            let inset = 3.0;
            let dest = Rect::new(
                rect.x + inset,
                rect.y + inset,
                (rect.w - inset * 2.0).max(1.0),
                (rect.h - inset * 2.0).max(1.0),
            );
            self.draw
                .tex_rect(tex, dest, uv, Color::rgb(1.0, 1.0, 1.0));
        }
        if let Some(n) = count.filter(|n| *n > 1) {
            let label = n.to_string();
            let size_t = (self.theme.typography.label * 0.85).max(10.0);
            let tw = self.measure_width(&label, size_t);
            self.draw.text(
                rect.x + rect.w - tw - 3.0,
                rect.y + rect.h - size_t - 2.0,
                size_t,
                self.theme.colors.text,
                &label,
            );
        }
        self.access(
            AccessNode::new(id, Role::Button)
                .label("slot")
                .value(count.map(|n| n.to_string()).unwrap_or_default())
                .rect(rect)
                .selected(selected)
                .focusable(true)
                .focused(response.focused),
        );
        response
    }

    /// 快捷栏：一排 `slot`，返回被点击的下标。
    pub fn hotbar(
        &mut self,
        salt: impl std::hash::Hash,
        slot_size: f32,
        selected: usize,
        count: usize,
        mut paint: impl FnMut(usize) -> (Option<(TextureId, Rect)>, Option<u32>),
    ) -> Option<usize> {
        let count = count.max(1);
        let gap = self.theme.spacing.sm;
        let total_w = count as f32 * slot_size + gap * (count.saturating_sub(1) as f32);
        let row = self.allocate(slot_size, Some(total_w.min(self.available_rect().w)));
        let id = self.id_from(("hotbar", salt));
        let mut clicked = None;
        for i in 0..count {
            let x = row.x + i as f32 * (slot_size + gap);
            let cell = Rect::new(x, row.y, slot_size, slot_size);
            self.layouts
                .push(LayoutCursor::new(cell, Layout::vertical()));
            let (icon, stack) = paint(i);
            let response =
                self.slot(("hotbar_slot", id.raw(), i), slot_size, icon, stack, i == selected);
            if response.clicked {
                clicked = Some(i);
            }
            self.layouts.pop();
        }
        clicked
    }
}
