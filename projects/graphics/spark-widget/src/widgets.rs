//! 常用补充控件：进度条、分隔线、折叠、单选、页签。

use spark_core::{Color, Rect};

use crate::accessibility::{AccessNode, Role};
use crate::layout::Layout;
use crate::response::Response;
use crate::style::TextTone;
use crate::ui::Ui;

impl Ui<'_> {
    pub fn separator(&mut self) -> Response {
        let h = 8.0;
        let rect = self.allocate(h, None);
        let id = self.id_from("separator");
        let line = Rect::new(rect.x, rect.y + h * 0.5 - 1.0, rect.w, 2.0);
        self.draw.fill_rect(line, self.theme.colors.track);
        self.access(AccessNode::new(id, Role::Separator).rect(rect));
        Response::empty(id, rect)
    }

    pub fn progress_bar(&mut self, value: f32) -> Response {
        let height = self.theme.metrics.slider_height;
        let rect = self.allocate(height, None);
        let id = self.id_from("progress");
        let t = value.clamp(0.0, 1.0);
        self.draw.fill_rect(rect, self.theme.colors.track);
        if t > 0.0 {
            self.draw.fill_rect(
                Rect::new(rect.x, rect.y, rect.w * t, rect.h),
                self.theme.colors.knob,
            );
        }
        self.access(
            AccessNode::new(id, Role::ProgressBar)
                .rect(rect)
                .value(format!("{:.0}%", t * 100.0)),
        );
        Response::empty(id, rect)
    }

    /// 折叠面板。`open` 由调用方跨帧持有。
    pub fn collapsible<R>(
        &mut self,
        title: impl AsRef<str>,
        open: &mut bool,
        f: impl FnOnce(&mut Self) -> R,
    ) -> (Response, Option<R>) {
        let title = title.as_ref();
        let height = self.theme.metrics.row_height;
        let rect = self.allocate(height, None);
        let id = self.id_from(("collapsible", title));
        self.state.register_focusable(id, rect);
        let mut response = self.interact(id, rect, true);
        if response.clicked {
            *open = !*open;
            response.changed = true;
        }
        let fill = if response.hovered {
            self.theme.colors.primary
        } else {
            self.theme.colors.panel
        };
        self.draw.fill_rect(rect, fill);
        let mark = if *open { "▼" } else { "▶" };
        self.draw.text(
            rect.x + 8.0,
            rect.y + (rect.h - self.theme.typography.label) * 0.5,
            self.theme.typography.label,
            self.theme.colors.text,
            &format!("{mark} {title}"),
        );
        self.access(
            AccessNode::new(id, Role::Button)
                .label(title.to_string())
                .rect(rect)
                .expanded(*open)
                .focusable(true)
                .focused(response.focused),
        );
        let body = if *open {
            Some(self.scope(("collapsible_body", title), f))
        } else {
            None
        };
        (response, body)
    }

    /// 单选：同组内只有一个为真。`group` 用于 ID 与互斥。
    pub fn radio(&mut self, group: impl AsRef<str>, label: impl AsRef<str>, selected: &mut bool) -> Response {
        let group = group.as_ref();
        let label = label.as_ref();
        let height = self.theme.metrics.row_height;
        let rect = self.allocate(height, None);
        let id = self.id_from(("radio", group, label));
        self.state.register_focusable(id, rect);
        let mut response = self.interact(id, rect, true);
        if response.clicked {
            *selected = true;
            response.changed = true;
        }
        let r = self.theme.metrics.checkbox.min(rect.h) * 0.5;
        let cx = rect.x + r + 2.0;
        let cy = rect.y + rect.h * 0.5;
        let outer = Rect::new(cx - r, cy - r, r * 2.0, r * 2.0);
        self.draw.fill_rect(outer, self.theme.colors.border);
        self.draw.fill_rect(
            Rect::new(outer.x + 2.0, outer.y + 2.0, outer.w - 4.0, outer.h - 4.0),
            Color::rgb(0.10, 0.14, 0.22),
        );
        if *selected {
            let ir = r * 0.45;
            self.draw.fill_rect(
                Rect::new(cx - ir, cy - ir, ir * 2.0, ir * 2.0),
                self.theme.colors.checkbox_on,
            );
        }
        self.draw.text(
            outer.x + outer.w + 8.0,
            rect.y + (rect.h - self.theme.typography.label) * 0.5,
            self.theme.typography.label,
            self.theme.colors.text,
            label,
        );
        self.access(
            AccessNode::new(id, Role::Radio)
                .label(label.to_string())
                .rect(rect)
                .checked(*selected)
                .focusable(true)
                .focused(response.focused),
        );
        response
    }

    /// 页签条。返回本帧选中下标是否变化。
    pub fn tabs(&mut self, titles: &[&str], selected: &mut usize) -> Response {
        if titles.is_empty() {
            return Response::empty(self.id_from("tabs_empty"), self.available_rect());
        }
        *selected = (*selected).min(titles.len() - 1);
        let height = self.theme.metrics.button_height;
        let total = self.allocate(height, None);
        let tab_w = total.w / titles.len() as f32;
        let bar_id = self.id_from("tabs");
        let mut changed = false;
        let mut hovered = false;
        let mut focused = false;
        for (i, title) in titles.iter().enumerate() {
            let rect = Rect::new(total.x + i as f32 * tab_w, total.y, tab_w - 2.0, total.h);
            let id = self.id_from(("tab", *title, i));
            self.state.register_focusable(id, rect);
            let response = self.interact(id, rect, true);
            hovered |= response.hovered;
            focused |= response.focused;
            if response.clicked && *selected != i {
                *selected = i;
                changed = true;
            }
            let on = *selected == i;
            let fill = if on {
                self.theme.colors.primary_hover
            } else if response.hovered {
                self.theme.colors.primary
            } else {
                self.theme.colors.panel
            };
            self.draw.fill_rect(rect, fill);
            let size = self.theme.typography.label;
            self.draw.text(
                rect.x + 8.0,
                rect.y + (rect.h - size) * 0.5,
                size,
                self.theme.colors.text,
                *title,
            );
            self.access(
                AccessNode::new(id, Role::Tab)
                    .label((*title).to_string())
                    .rect(rect)
                    .selected(on)
                    .focusable(true)
                    .focused(response.focused),
            );
        }
        Response {
            id: bar_id,
            rect: total,
            hovered,
            active: false,
            focused,
            clicked: false,
            changed,
            double_clicked: false,
        }
    }

    /// 打开时绘制 modal 内容区；关闭时返回 `None`。
    pub fn modal<R>(
        &mut self,
        salt: impl std::hash::Hash,
        title: &str,
        f: impl FnOnce(&mut Self) -> R,
    ) -> Option<(Response, R)> {
        let id = self.id_from(("modal", salt));
        if self.state.overlays.open_modal != Some(id) {
            return None;
        }
        let viewport = self.viewport;
        let w = (viewport.w * 0.5).clamp(280.0, 520.0);
        let h = (viewport.h * 0.45).clamp(160.0, 420.0);
        let panel = Rect::new(
            viewport.x + (viewport.w - w) * 0.5,
            viewport.y + (viewport.h - h) * 0.5,
            w,
            h,
        );
        // 遮罩在 overlay flush 也会画；这里先画一层保证内容在同帧可见。
        self.draw
            .fill_rect(viewport, Color::rgba(0.0, 0.0, 0.0, 0.55));
        self.draw.fill_rect(panel, self.theme.colors.panel);
        self.draw.fill_rect(
            Rect::new(panel.x, panel.y, panel.w, 32.0),
            self.theme.colors.panel_title,
        );
        self.draw.text(
            panel.x + 12.0,
            panel.y + 6.0,
            self.theme.typography.label,
            self.theme.colors.text,
            title,
        );
        let content = Rect::new(
            panel.x + 12.0,
            panel.y + 40.0,
            (panel.w - 24.0).max(1.0),
            (panel.h - 52.0).max(1.0),
        );
        self.layouts
            .push(crate::layout::LayoutCursor::new(content, Layout::vertical().gap(8.0)));
        let out = self.scope(("modal_body", id.raw()), f);
        self.layouts.pop();
        self.state.overlays.modal_body_drawn = true;
        self.state.memory_mut(id).last_rect = Some(panel);
        self.access(
            AccessNode::new(id, Role::Dialog)
                .label(title.to_string())
                .rect(panel),
        );
        let response = Response::empty(id, panel);
        Some((response, out))
    }

    pub fn muted(&mut self, text: impl AsRef<str>) -> Response {
        self.label_tone(text, TextTone::Muted)
    }
}
