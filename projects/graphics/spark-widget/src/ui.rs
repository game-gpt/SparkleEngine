//! 每帧 UI 上下文：输入、绘制、状态、主题与布局栈。

use std::sync::Arc;

use spark_core::{Color, Rect, Vec2};
use spark_input::{Input, Key, MouseBtn};
use spark_localization::LocaleSnapshot;
use spark_renderer::DrawList;

use crate::id::{IdStack, WidgetId};
use crate::layout::{Direction, Layout, LayoutCursor, Size};
use crate::response::Response;
use crate::state::{FocusSource, UiState};
use crate::style::{ButtonVariant, InteractState, TextTone, Theme};
use crate::text_source::TextSource;

/// 本帧时间。动效与双击判定可用。
#[derive(Debug, Clone, Copy, Default)]
pub struct UiTime {
    pub dt: f32,
    pub seconds: f64,
}

/// 统一 UI 上下文。
pub struct Ui<'a> {
    pub(crate) input: &'a Input,
    pub(crate) draw: &'a mut DrawList,
    pub(crate) state: &'a mut UiState,
    pub(crate) locale: Option<&'a LocaleSnapshot>,
    pub(crate) theme: Theme,
    pub(crate) viewport: Rect,
    pub(crate) time: UiTime,
    ids: IdStack,
    layouts: Vec<LayoutCursor>,
    enabled: bool,
}

/// 构造参数。
pub struct UiBuilder<'a> {
    pub input: &'a Input,
    pub draw: &'a mut DrawList,
    pub state: &'a mut UiState,
    pub theme: Theme,
    pub viewport: Rect,
    pub time: UiTime,
    pub locale: Option<&'a LocaleSnapshot>,
}

impl<'a> Ui<'a> {
    pub fn new(builder: UiBuilder<'a>) -> Self {
        let mut ui = Self {
            input: builder.input,
            draw: builder.draw,
            state: builder.state,
            locale: builder.locale,
            theme: builder.theme,
            viewport: builder.viewport,
            time: builder.time,
            ids: IdStack::new(),
            layouts: Vec::new(),
            enabled: true,
        };
        ui.state.begin_frame();
        let root = LayoutCursor::new(builder.viewport, Layout::vertical().width(Size::Fill));
        ui.layouts.push(root);
        let _ = ui.state.motion.tick(ui.time.dt);
        ui
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn input(&self) -> &Input {
        self.input
    }

    pub fn draw(&mut self) -> &mut DrawList {
        self.draw
    }

    pub fn state(&mut self) -> &mut UiState {
        self.state
    }

    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    /// 结束本帧：Tab 导航；指针抬起时清除 active / capture。
    pub fn end(mut self) {
        self.handle_focus_keys();
        if !self.input.mouse_down(MouseBtn::Left) {
            self.state.active = None;
            self.state.captured = None;
        }
    }

    pub fn scope<R>(&mut self, salt: impl std::hash::Hash, f: impl FnOnce(&mut Self) -> R) -> R {
        self.ids.push_hashable(salt);
        let out = f(self);
        self.ids.pop();
        out
    }

    pub fn push_id<R>(&mut self, salt: impl std::hash::Hash, f: impl FnOnce(&mut Self) -> R) -> R {
        self.scope(salt, f)
    }

    pub fn next_id(&mut self) -> WidgetId {
        let id = self.ids.push_auto();
        self.ids.pop();
        self.state.note_id(id);
        id
    }

    pub fn id_from(&mut self, salt: impl std::hash::Hash) -> WidgetId {
        let id = self.ids.push_hashable(salt);
        self.ids.pop();
        self.state.note_id(id);
        id
    }

    pub fn column<R>(&mut self, layout: Layout, f: impl FnOnce(&mut Self) -> R) -> R {
        let mut layout = layout;
        layout.direction = Direction::Vertical;
        self.with_layout(layout, f)
    }

    pub fn row<R>(&mut self, layout: Layout, f: impl FnOnce(&mut Self) -> R) -> R {
        let mut layout = layout;
        layout.direction = Direction::Horizontal;
        self.with_layout(layout, f)
    }

    pub fn with_layout<R>(&mut self, layout: Layout, f: impl FnOnce(&mut Self) -> R) -> R {
        let parent = self.layouts.last().expect("layout stack");
        let bounds = match layout.direction {
            Direction::Vertical => {
                let h = match layout.height {
                    Size::Px(px) => px,
                    Size::Percent(p) => parent.content.h * p.clamp(0.0, 1.0),
                    Size::Fill | Size::Auto => parent.remaining_main(),
                };
                let mut cursor = parent.clone();
                let rect = cursor.allocate(h, None);
                *self.layouts.last_mut().unwrap() = cursor;
                rect
            }
            Direction::Horizontal => {
                let w = match layout.width {
                    Size::Px(px) => px,
                    Size::Percent(p) => parent.content.w * p.clamp(0.0, 1.0),
                    Size::Fill | Size::Auto => parent.remaining_main(),
                };
                let mut cursor = parent.clone();
                let rect = cursor.allocate(w, None);
                *self.layouts.last_mut().unwrap() = cursor;
                rect
            }
        };
        self.layouts.push(LayoutCursor::new(bounds, layout));
        let out = f(self);
        self.layouts.pop();
        out
    }

    pub fn available_rect(&self) -> Rect {
        self.layouts
            .last()
            .map(|c| c.content)
            .unwrap_or(self.viewport)
    }

    pub fn allocate(&mut self, main: f32, cross: Option<f32>) -> Rect {
        self.layouts
            .last_mut()
            .expect("layout stack")
            .allocate(main, cross)
    }

    pub fn spacer(&mut self, main: f32) {
        let _ = self.allocate(main, None);
    }

    pub fn request_focus(&mut self, id: WidgetId) {
        self.state.request_focus(id, FocusSource::Programmatic);
    }

    pub fn focus_next(&mut self) {
        self.move_focus(1);
    }

    pub fn focus_previous(&mut self) {
        self.move_focus(-1);
    }

    fn move_focus(&mut self, delta: i32) {
        let order = &self.state.focus_order;
        if order.is_empty() {
            return;
        }
        let current = self
            .state
            .focused
            .and_then(|id| order.iter().position(|x| *x == id))
            .unwrap_or(0);
        let len = order.len() as i32;
        let next = (current as i32 + delta).rem_euclid(len) as usize;
        let id = order[next];
        self.state.request_focus(id, FocusSource::Keyboard);
    }

    fn handle_focus_keys(&mut self) {
        if self.input.key_pressed(Key::Tab) {
            if self.input.key_down(Key::LShift) || self.input.key_down(Key::RShift) {
                self.focus_previous();
            } else {
                self.focus_next();
            }
        }
    }

    /// 命中测试并更新 hot / active / capture。
    pub fn interact(&mut self, id: WidgetId, rect: Rect, sense_click: bool) -> Response {
        self.state.note_id(id);
        self.state.memory_mut(id).last_rect = Some(rect);

        let (mx, my) = self.input.mouse_pos();
        let pointer = Vec2::new(mx, my);
        let pointer_in = rect.contains(pointer) && self.enabled;

        let captured = self.state.captured == Some(id);
        let hovered = if let Some(cap) = self.state.captured {
            cap == id
        } else {
            pointer_in
        };

        if hovered && self.enabled {
            self.state.hot = Some(id);
        }

        if sense_click && self.enabled && pointer_in && self.input.mouse_pressed(MouseBtn::Left) {
            self.state.active = Some(id);
            self.state.request_focus(id, FocusSource::Pointer);
        }

        if self.state.active == Some(id) && self.input.mouse_down(MouseBtn::Left) && !pointer_in {
            // 按下后移出仍保持 active，供捕获使用。
        }

        let clicked = self.enabled
            && self.state.active == Some(id)
            && hovered
            && self.input.mouse_released(MouseBtn::Left);

        if captured && self.input.mouse_released(MouseBtn::Left) {
            self.state.captured = None;
        }

        let focused = self.state.focused == Some(id);
        let active = self.state.active == Some(id);

        Response {
            id,
            rect,
            hovered,
            active,
            focused,
            clicked,
            changed: false,
            double_clicked: false,
        }
    }

    pub fn capture(&mut self, id: WidgetId) {
        self.state.captured = Some(id);
        self.state.active = Some(id);
    }

    pub fn interact_state(&self, response: &Response) -> InteractState {
        if !self.enabled {
            return InteractState::Disabled;
        }
        if response.active {
            InteractState::Pressed
        } else if response.hovered {
            InteractState::Hovered
        } else if response.focused {
            InteractState::Focused
        } else {
            InteractState::Normal
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn with_enabled<R>(&mut self, enabled: bool, f: impl FnOnce(&mut Self) -> R) -> R {
        let prev = self.enabled;
        self.enabled = enabled;
        let out = f(self);
        self.enabled = prev;
        out
    }

    pub fn label(&mut self, text: impl AsRef<str>) -> Response {
        self.label_tone(text, TextTone::Primary)
    }

    pub fn label_tone(&mut self, text: impl AsRef<str>, tone: TextTone) -> Response {
        let text = text.as_ref();
        let size = self.theme.typography.label;
        let height = size + 4.0;
        let rect = self.allocate(height, None);
        let id = self.id_from(("label", text));
        let color = self.theme.text_color(tone);
        self.draw.text(rect.x, rect.y + 2.0, size, color, text);
        Response::empty(id, rect)
    }

    pub fn heading(&mut self, text: impl AsRef<str>) -> Response {
        let text = text.as_ref();
        let size = self.theme.typography.heading;
        let height = size + 8.0;
        let rect = self.allocate(height, None);
        let id = self.id_from(("heading", text));
        self.draw
            .text(rect.x, rect.y + 2.0, size, self.theme.colors.text, text);
        Response::empty(id, rect)
    }

    pub fn button(&mut self, text: impl AsRef<str>) -> Response {
        self.button_variant(text, ButtonVariant::Primary)
    }

    pub fn button_variant(&mut self, text: impl AsRef<str>, variant: ButtonVariant) -> Response {
        let text = text.as_ref();
        let height = self.theme.metrics.button_height;
        let rect = self.allocate(height, None);
        let id = self.id_from(("button", text));
        self.state.register_focusable(id);
        let response = self.interact(id, rect, true);
        let state = self.interact_state(&response);
        let fill = self.theme.button_fill(variant, state);
        let border = if response.focused {
            self.theme.colors.focus_ring
        } else {
            self.theme.colors.border
        };
        self.draw.fill_rect(
            Rect::new(rect.x - 2.0, rect.y - 2.0, rect.w + 4.0, rect.h + 4.0),
            border,
        );
        self.draw.fill_rect(rect, fill);
        let size = self.theme.typography.button;
        let est_w = estimate_text_width(text, size);
        let tx = rect.x + (rect.w - est_w).max(0.0) * 0.5;
        let ty = rect.y + (rect.h - size) * 0.5;
        self.draw
            .text(tx, ty, size, self.theme.colors.text, text);
        response
    }

    pub fn checkbox(&mut self, label: impl AsRef<str>, checked: &mut bool) -> Response {
        let label = label.as_ref();
        let height = self.theme.metrics.row_height;
        let rect = self.allocate(height, None);
        let id = self.id_from(("checkbox", label));
        self.state.register_focusable(id);
        let mut response = self.interact(id, rect, true);
        let box_s = self.theme.metrics.checkbox.min(rect.h);
        let box_r = Rect::new(rect.x, rect.y + (rect.h - box_s) * 0.5, box_s, box_s);
        if response.clicked {
            *checked = !*checked;
            response.changed = true;
        }
        self.draw.fill_rect(
            Rect::new(box_r.x - 1.0, box_r.y - 1.0, box_r.w + 2.0, box_r.h + 2.0),
            self.theme.colors.border,
        );
        self.draw
            .fill_rect(box_r, Color::rgb(0.10, 0.14, 0.22));
        if *checked {
            let inset = 4.0;
            self.draw.fill_rect(
                Rect::new(
                    box_r.x + inset,
                    box_r.y + inset,
                    box_r.w - inset * 2.0,
                    box_r.h - inset * 2.0,
                ),
                self.theme.colors.checkbox_on,
            );
        }
        self.draw.text(
            box_r.x + box_s + 8.0,
            rect.y + (rect.h - self.theme.typography.label) * 0.5,
            self.theme.typography.label,
            self.theme.colors.text,
            label,
        );
        response
    }

    pub fn slider(&mut self, value: &mut f32, range: std::ops::RangeInclusive<f32>) -> Response {
        let min = *range.start();
        let max = (*range.end()).max(min + 1e-6);
        let height = self.theme.metrics.slider_height;
        let rect = self.allocate(height, None);
        let id = self.id_from("slider");
        self.state.register_focusable(id);
        let mut response = self.interact(id, rect, true);
        if response.hovered && self.input.mouse_pressed(MouseBtn::Left) {
            self.capture(id);
        }
        let dragging = self.state.captured == Some(id) && self.input.mouse_down(MouseBtn::Left);
        if dragging {
            let (mx, _) = self.input.mouse_pos();
            let t = ((mx - rect.x) / rect.w).clamp(0.0, 1.0);
            let nv = min + t * (max - min);
            if (nv - *value).abs() > 1e-6 {
                *value = nv;
                response.changed = true;
            }
            response.active = true;
        }
        let track_h = 6.0;
        let track = Rect::new(
            rect.x,
            rect.y + (rect.h - track_h) * 0.5,
            rect.w,
            track_h,
        );
        self.draw.fill_rect(track, self.theme.colors.track);
        let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
        let knob_x = rect.x + t * rect.w;
        let knob = Rect::new(knob_x - 6.0, rect.y + 2.0, 12.0, rect.h - 4.0);
        self.draw.fill_rect(knob, self.theme.colors.knob);
        response
    }

    pub fn panel<R>(&mut self, title: &str, f: impl FnOnce(&mut Self) -> R) -> R {
        let remaining = self
            .layouts
            .last()
            .map(|c| c.remaining_main())
            .unwrap_or(200.0);
        let bounds = self.allocate(remaining.max(80.0), None);
        self.draw.fill_rect(bounds, self.theme.colors.panel);
        let bar_h = 28.0;
        self.draw.fill_rect(
            Rect::new(bounds.x, bounds.y, bounds.w, bar_h),
            self.theme.colors.panel_title,
        );
        self.draw.text(
            bounds.x + 10.0,
            bounds.y + 4.0,
            self.theme.typography.label,
            self.theme.colors.text,
            title,
        );
        let content = Rect::new(
            bounds.x + 8.0,
            bounds.y + bar_h + 8.0,
            (bounds.w - 16.0).max(1.0),
            (bounds.h - bar_h - 16.0).max(1.0),
        );
        self.layouts
            .push(LayoutCursor::new(content, Layout::vertical().gap(self.theme.spacing.sm)));
        let out = self.scope(title, f);
        self.layouts.pop();
        out
    }

    pub fn label_source(&mut self, source: &TextSource) -> Response {
        let text = if let Some(locale) = self.locale {
            source.resolve(locale).text
        } else {
            match source {
                TextSource::Literal(s) => s.clone(),
                TextSource::Message { id, .. } => Arc::from(format!("{id:?}")),
            }
        };
        self.label(text.as_ref())
    }

    pub fn list_row(&mut self, text: impl AsRef<str>) -> Response {
        let text = text.as_ref();
        let height = self.theme.metrics.row_height;
        let rect = self.allocate(height, None);
        let id = self.id_from(("row", text));
        self.state.register_focusable(id);
        let response = self.interact(id, rect, true);
        let fill = if response.active || response.focused {
            self.theme.colors.primary_hover
        } else if response.hovered {
            self.theme.colors.primary
        } else {
            Color::rgb(0.08, 0.12, 0.20)
        };
        self.draw.fill_rect(rect, fill);
        self.draw.text(
            rect.x + 8.0,
            rect.y + (rect.h - self.theme.typography.label) * 0.5,
            self.theme.typography.label,
            self.theme.colors.text,
            text,
        );
        response
    }
}

fn estimate_text_width(text: &str, size: f32) -> f32 {
    // 临时估算：中文按全角，ASCII 按 0.55em。正式测量交给字体子系统接入。
    let mut w = 0.0;
    for ch in text.chars() {
        if ch.is_ascii() {
            w += size * 0.55;
        } else {
            w += size;
        }
    }
    w
}
