//! 弹出菜单与右键上下文菜单。

use std::hash::Hash;

use spark_core::{Rect, Vec2};
use spark_input::{Key, MouseBtn};

use crate::accessibility::{AccessNode, Role};
use crate::response::Response;
use crate::ui::Ui;

impl Ui<'_> {
    /// 在锚点下方弹出菜单。`open` 由调用方跨帧持有；返回本帧选中的项下标。
    pub fn popup_menu(
        &mut self,
        salt: impl Hash,
        anchor: Rect,
        open: &mut bool,
        items: &[&str],
    ) -> Option<usize> {
        if !*open || items.is_empty() {
            return None;
        }
        let id = self.id_from(("popup", salt));
        let row_h = self.theme.metrics.row_height;
        let width = anchor.w.max(120.0);
        let height = row_h * items.len() as f32;
        let mut menu = Rect::new(anchor.x, anchor.y + anchor.h + 2.0, width, height);
        // 贴边：必要时翻到锚点上方或左移
        if menu.x + menu.w > self.viewport.x + self.viewport.w {
            menu.x = (self.viewport.x + self.viewport.w - menu.w).max(self.viewport.x);
        }
        if menu.y + menu.h > self.viewport.y + self.viewport.h {
            menu.y = (anchor.y - height - 2.0).max(self.viewport.y);
        }

        self.draw.fill_rect(menu, self.theme.colors.panel);
        self.draw.fill_rect(
            Rect::new(menu.x - 1.0, menu.y - 1.0, menu.w + 2.0, menu.h + 2.0),
            self.theme.colors.border,
        );

        let mut chosen = None;
        let size = self.theme.typography.label;
        for (i, item) in items.iter().enumerate() {
            let row = Rect::new(menu.x, menu.y + i as f32 * row_h, menu.w, row_h);
            let row_id = self.id_from(("popup_item", id.raw(), i));
            self.state.register_focusable(row_id, row);
            let mut response = self.interact(row_id, row, true);
            if self.keyboard_activate(row_id) {
                response.clicked = true;
            }
            let fill = if response.hovered || response.focused {
                self.theme.colors.primary_hover
            } else {
                self.theme.colors.panel
            };
            self.draw.fill_rect(row, fill);
            self.draw.text(
                row.x + 8.0,
                row.y + (row.h - size) * 0.5,
                size,
                self.theme.colors.text,
                *item,
            );
            self.access(
                AccessNode::new(row_id, Role::Button)
                    .label((*item).to_string())
                    .rect(row)
                    .focusable(true)
                    .focused(response.focused),
            );
            if response.clicked {
                chosen = Some(i);
            }
        }

        if self.input.key_pressed(Key::Escape) {
            *open = false;
        }
        if self.input.mouse_pressed(MouseBtn::Left) || self.input.mouse_pressed(MouseBtn::Right) {
            let (mx, my) = self.input.mouse_pos();
            let p = Vec2::new(mx, my);
            if !menu.contains(p) && !anchor.contains(p) {
                *open = false;
            }
        }
        if chosen.is_some() {
            *open = false;
        }

        self.access(
            AccessNode::new(id, Role::Dialog)
                .label("popup")
                .rect(menu)
                .expanded(true),
        );
        chosen
    }

    /// 在 `host` 上右键打开上下文菜单；返回选中项下标。
    pub fn context_menu(
        &mut self,
        salt: impl Hash,
        host: &Response,
        items: &[&str],
    ) -> Option<usize> {
        let id = self.id_from(("context", salt));
        let mut open = self.state.memory(id).map(|m| m.opened).unwrap_or(false);
        if host.hovered && self.input.mouse_pressed(MouseBtn::Right) {
            open = true;
        }
        let chosen = self.popup_menu(("context_body", id.raw()), host.rect, &mut open, items);
        self.state.memory_mut(id).opened = open;
        chosen
    }
}
