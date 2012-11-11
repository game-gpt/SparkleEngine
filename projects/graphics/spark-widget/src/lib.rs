//! 立即模式 Widget 系统。
//!
//! 推荐入口是 [`Ui`]：统一 ID、状态、命中、主题与布局。
//! 底部的 `panel` / `label` / `button` 等自由函数仍可用，但新页面应走 `Ui`。
//!
//! 界面控件叫 **Widget**，避免与 ECS `Component` 混淆。
//! UI 缓动在 [`motion`]，不属于 `spark-animator`。

mod accessibility;
mod containers;
mod debug;
mod drag_drop;
mod fields;
mod focus;
mod game_widgets;
mod id;
mod inspector;
mod layout;
pub mod motion;
mod overlay;
mod popup;
mod prefs;
mod response;
mod rich_text;
mod scroll;
mod select;
mod state;
mod style;
mod table;
mod text;
mod text_source;
mod ui;
mod virtual_list;
mod widgets;

pub use accessibility::{AccessNode, AccessTree, Role};
pub use debug::UiDebug;
pub use drag_drop::{DragPayload, DragState};
pub use game_widgets::TreeNode;
pub use id::WidgetId;
pub use inspector::UiInspector;
pub use layout::{Align, Direction, GridState, Insets, Justify, Layout, LayoutCursor, Size};
pub use motion::{
    Easing, MotionId, MotionProperty, MotionScheduler, MotionSequence, MotionSpec, MotionTick,
    MotionValue, SequenceStep, Spring, SpringParams, Transition, Tween,
};
pub use overlay::{OverlayState, ToastEntry};
pub use prefs::UiPrefs;
pub use response::Response;
pub use rich_text::{RichSpan, RichText};
pub use state::{FocusSource, UiState, WidgetMemory};
pub use style::{
    ButtonVariant, InteractState, MotionTheme, Spacing, TextTone, Theme, Typography, UiColors,
    WidgetMetrics,
};
pub use text::{wrap_lines, EstimateMeasurer, TextMeasurer};
pub use text_source::{ResolvedText, TextBinding, TextSource};
pub use ui::{Ui, UiBuilder, UiTime};

use spark_core::{Color, Rect, Vec2};
use spark_input::{Input, Key, MouseBtn};
use spark_localization::LocaleSnapshot;
use spark_renderer::DrawList;

/// 兼容旧 API 的焦点 ID。新代码请用 [`WidgetId`]。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct FocusId(pub u32);

#[derive(Debug, Clone, Default)]
pub struct FocusState {
    pub active: Option<FocusId>,
}

impl FocusState {
    pub fn is_active(&self, id: FocusId) -> bool {
        self.active == Some(id)
    }

    pub fn focus(&mut self, id: FocusId) {
        self.active = Some(id);
    }

    pub fn clear(&mut self) {
        self.active = None;
    }

    pub fn toggle(&mut self, id: FocusId) {
        if self.active == Some(id) {
            self.active = None;
        } else {
            self.active = Some(id);
        }
    }
}

/// 纵向流式布局光标（手动分配）。新代码优先 [`Ui::column`]。
#[derive(Debug, Clone)]
pub struct Column {
    pub origin: Vec2,
    pub width: f32,
    pub y: f32,
    pub gap: f32,
}

impl Column {
    pub fn new(origin: Vec2, width: f32) -> Self {
        Self {
            origin,
            width: width.max(1.0),
            y: origin.y,
            gap: 8.0,
        }
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.0);
        self
    }

    pub fn alloc(&mut self, height: f32) -> Rect {
        let h = height.max(1.0);
        let r = Rect::new(self.origin.x, self.y, self.width, h);
        self.y += h + self.gap;
        r
    }

    pub fn skip(&mut self, dy: f32) {
        self.y += dy;
    }
}

/// 横向流式布局光标（手动分配）。新代码优先 [`Ui::row`]。
#[derive(Debug, Clone)]
pub struct Row {
    pub origin: Vec2,
    pub height: f32,
    pub x: f32,
    pub gap: f32,
}

impl Row {
    pub fn new(origin: Vec2, height: f32) -> Self {
        Self {
            origin,
            height: height.max(1.0),
            x: origin.x,
            gap: 8.0,
        }
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.0);
        self
    }

    pub fn alloc(&mut self, width: f32) -> Rect {
        let w = width.max(1.0);
        let r = Rect::new(self.x, self.origin.y, w, self.height);
        self.x += w + self.gap;
        r
    }
}

pub fn panel(draw: &mut DrawList, rect: Rect, fill: Color) {
    draw.fill_rect(rect, fill);
}

pub fn titled_panel(
    draw: &mut DrawList,
    rect: Rect,
    title: &str,
    fill: Color,
    title_bar: Color,
) -> Rect {
    let bar_h = 28.0;
    draw.fill_rect(rect, fill);
    draw.fill_rect(Rect::new(rect.x, rect.y, rect.w, bar_h), title_bar);
    label(
        draw,
        rect.x + 10.0,
        rect.y + 4.0,
        18.0,
        Color::rgb(0.95, 0.97, 1.0),
        title,
    );
    Rect::new(
        rect.x + 8.0,
        rect.y + bar_h + 8.0,
        (rect.w - 16.0).max(1.0),
        (rect.h - bar_h - 16.0).max(1.0),
    )
}

pub fn label(draw: &mut DrawList, x: f32, y: f32, size: f32, color: Color, text: &str) {
    draw.text(x, y, size, color, text);
}

pub fn label_source(
    draw: &mut DrawList,
    snapshot: &LocaleSnapshot,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    source: &TextSource,
) {
    let resolved = source.resolve(snapshot);
    draw.text(x, y, size, color, resolved.text.as_ref());
}

pub fn button(draw: &mut DrawList, input: &Input, rect: Rect, text: &str) -> bool {
    let (mx, my) = input.mouse_pos();
    let hovered = rect.contains(Vec2::new(mx, my));
    let pressed = hovered && input.mouse_down(MouseBtn::Left);
    let clicked = hovered && input.mouse_pressed(MouseBtn::Left);

    let fill = if pressed {
        Color::rgb(0.20, 0.45, 0.75)
    } else if hovered {
        Color::rgb(0.18, 0.35, 0.58)
    } else {
        Color::rgb(0.12, 0.22, 0.38)
    };
    let border = Color::rgb(0.45, 0.70, 0.95);
    draw.fill_rect(
        Rect::new(rect.x - 2.0, rect.y - 2.0, rect.w + 4.0, rect.h + 4.0),
        border,
    );
    draw.fill_rect(rect, fill);

    let size = 22.0;
    let est_w = text.chars().count() as f32 * size * 0.55;
    let tx = rect.x + (rect.w - est_w).max(0.0) * 0.5;
    let ty = rect.y + (rect.h - size) * 0.5;
    draw.text(tx, ty, size, Color::rgb(0.92, 0.96, 1.0), text);

    clicked
}

pub fn list_row(
    draw: &mut DrawList,
    input: &Input,
    focus: &mut FocusState,
    id: FocusId,
    rect: Rect,
    text: &str,
) -> bool {
    let (mx, my) = input.mouse_pos();
    let hovered = rect.contains(Vec2::new(mx, my));
    if hovered && input.mouse_pressed(MouseBtn::Left) {
        focus.focus(id);
    }
    let active = focus.is_active(id);
    let fill = if active {
        Color::rgb(0.22, 0.40, 0.62)
    } else if hovered {
        Color::rgb(0.14, 0.24, 0.40)
    } else {
        Color::rgb(0.08, 0.12, 0.20)
    };
    draw.fill_rect(rect, fill);
    label(
        draw,
        rect.x + 8.0,
        rect.y + (rect.h - 18.0) * 0.5,
        18.0,
        Color::rgb(0.92, 0.95, 1.0),
        text,
    );
    let activated = active && (input.key_pressed(Key::Enter) || input.key_pressed(Key::Space));
    let clicked = hovered && input.mouse_pressed(MouseBtn::Left);
    activated || clicked
}

pub fn checkbox(
    draw: &mut DrawList,
    input: &Input,
    rect: Rect,
    label_text: &str,
    checked: &mut bool,
) -> bool {
    let box_s = rect.h.min(22.0);
    let box_r = Rect::new(rect.x, rect.y + (rect.h - box_s) * 0.5, box_s, box_s);
    let hit = Rect::new(rect.x, rect.y, rect.w, rect.h);
    let (mx, my) = input.mouse_pos();
    let hovered = hit.contains(Vec2::new(mx, my));
    let toggled = hovered && input.mouse_pressed(MouseBtn::Left);
    if toggled {
        *checked = !*checked;
    }
    draw.fill_rect(
        Rect::new(box_r.x - 1.0, box_r.y - 1.0, box_r.w + 2.0, box_r.h + 2.0),
        Color::rgb(0.45, 0.70, 0.95),
    );
    draw.fill_rect(box_r, Color::rgb(0.10, 0.14, 0.22));
    if *checked {
        let inset = 4.0;
        draw.fill_rect(
            Rect::new(
                box_r.x + inset,
                box_r.y + inset,
                box_r.w - inset * 2.0,
                box_r.h - inset * 2.0,
            ),
            Color::rgb(0.35, 0.75, 0.55),
        );
    }
    label(
        draw,
        box_r.x + box_s + 8.0,
        rect.y + (rect.h - 18.0) * 0.5,
        18.0,
        Color::rgb(0.9, 0.93, 1.0),
        label_text,
    );
    toggled
}

pub fn slider(
    draw: &mut DrawList,
    input: &Input,
    rect: Rect,
    min: f32,
    max: f32,
    value: &mut f32,
) -> bool {
    let max = max.max(min + 1e-6);
    let track_h = 6.0;
    let track = Rect::new(
        rect.x,
        rect.y + (rect.h - track_h) * 0.5,
        rect.w,
        track_h,
    );
    draw.fill_rect(track, Color::rgb(0.12, 0.16, 0.24));
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    let knob_x = rect.x + t * rect.w;
    let knob = Rect::new(knob_x - 6.0, rect.y + 2.0, 12.0, rect.h - 4.0);
    let (mx, my) = input.mouse_pos();
    let hovering = rect.contains(Vec2::new(mx, my));
    let dragging = hovering && input.mouse_down(MouseBtn::Left);
    let mut changed = false;
    if dragging {
        let nt = ((mx - rect.x) / rect.w).clamp(0.0, 1.0);
        let nv = min + nt * (max - min);
        if (nv - *value).abs() > 1e-6 {
            *value = nv;
            changed = true;
        }
    }
    draw.fill_rect(knob, Color::rgb(0.45, 0.75, 0.95));
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_core::Color;
    use spark_input::{ButtonState, Input};
    use spark_renderer::DrawList;

    #[test]
    fn column_allocates_downward() {
        let mut col = Column::new(Vec2::new(10.0, 20.0), 100.0).with_gap(4.0);
        let a = col.alloc(30.0);
        let b = col.alloc(30.0);
        assert!((a.y - 20.0).abs() < 1e-5);
        assert!((b.y - (20.0 + 30.0 + 4.0)).abs() < 1e-5);
        assert!((a.w - 100.0).abs() < 1e-5);
    }

    #[test]
    fn ui_button_click_on_release() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);

        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let response = ui.button("开始");
            assert!(response.active);
            assert!(!response.clicked);
            ui.end();
            assert!(state.active.is_some());
        }

        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            input.begin_frame();
            input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let response = ui.button("开始");
            assert!(response.clicked, "expected click on release");
            ui.end();
            assert!(state.active.is_none());
        }
    }

    #[test]
    fn ui_slider_keeps_dragging_outside_rect() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut value = 0.0_f32;

        {
            let mut input = Input::default();
            input.on_cursor(20.0, 10.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let response = ui.slider(&mut value, 0.0..=1.0);
            let slider_id = response.id;
            let active = response.active;
            ui.end();
            assert!(state.captured == Some(slider_id) || active || state.active == Some(slider_id));
        }

        {
            let mut input = Input::default();
            input.on_cursor(390.0, 10.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime {
                    dt: 0.016,
                    seconds: 0.0,
                },
                locale: None,
                text_measurer: None,
            });
            let response = ui.slider(&mut value, 0.0..=1.0);
            assert!(response.changed || value > 0.8, "value={value}");
            ui.end();
        }
    }

    #[test]
    fn scope_ids_are_stable() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let input = Input::default();
        let viewport = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut first = WidgetId::NONE;
        {
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            ui.scope("bag", |ui| {
                first = ui.id_from(7u32);
            });
            ui.end();
        }
        let mut second = WidgetId::NONE;
        {
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            ui.scope("bag", |ui| {
                second = ui.id_from(7u32);
            });
            ui.end();
        }
        assert_eq!(first, second);
        assert!(!first.is_none());
    }

    #[test]
    fn scroll_area_moves_with_wheel() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut input = Input::default();
        input.on_cursor(40.0, 40.0);
        input.on_wheel(-2.0);
        let mut ui = Ui::new(UiBuilder {
            input: &input,
            draw: &mut draw,
            state: &mut state,
            theme: Theme::default(),
            viewport,
            time: UiTime::default(),
            locale: None,
            text_measurer: None,
        });
        let (response, _) = ui.scroll_area("list", 120.0, |ui| {
            for i in 0..20 {
                ui.label(format!("row {i}"));
            }
        });
        let id = response.id;
        ui.end();
        let scroll = state.memory(id).map(|m| m.scroll.y).unwrap_or(0.0);
        assert!(scroll > 0.0, "scroll={scroll}");
    }

    #[test]
    fn tooltip_queues_when_hovered() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut input = Input::default();
        input.on_cursor(40.0, 20.0);
        let mut ui = Ui::new(UiBuilder {
            input: &input,
            draw: &mut draw,
            state: &mut state,
            theme: Theme::default(),
            viewport,
            time: UiTime::default(),
            locale: None,
            text_measurer: None,
        });
        let response = ui.button("提示");
        ui.tooltip(&response, "需要 10 个木材");
        let queued = !ui.state().overlays.queue.is_empty();
        ui.end();
        assert!(queued);
        assert!(!draw.texts.is_empty());
    }

    #[test]
    fn arrow_key_moves_focus_between_buttons() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        {
            let input = Input::default();
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let a = ui.button("A");
            let _b = ui.button("B");
            ui.request_focus(a.id);
            ui.end();
        }
        let focused_before = state.focused;
        {
            let mut input = Input::default();
            input.on_key(Key::Down, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let _a = ui.button("A");
            let b = ui.button("B");
            ui.end();
            assert_eq!(state.focused, Some(b.id));
            assert_ne!(state.focused, focused_before);
        }
    }

    #[test]
    fn text_field_inserts_and_backspaces() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut value = String::new();
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let _ = ui.text_field(&mut value);
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_text("Hi");
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime {
                    dt: 0.016,
                    seconds: 0.0,
                },
                locale: None,
                text_measurer: None,
            });
            let response = ui.text_field(&mut value);
            assert!(response.focused);
            assert!(response.changed);
            assert_eq!(value, "Hi");
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_key(Key::Backspace, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let _ = ui.text_field(&mut value);
            assert_eq!(value, "H");
            ui.end();
        }
    }

    #[test]
    fn wrap_lines_breaks_long_ascii() {
        let mut m = EstimateMeasurer;
        let lines = wrap_lines("hello world", 40.0, 20.0, &mut m);
        assert!(lines.len() >= 2, "{lines:?}");
    }

    #[test]
    fn virtual_list_only_visits_visible_rows() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let input = Input::default();
        let viewport = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut ui = Ui::new(UiBuilder {
            input: &input,
            draw: &mut draw,
            state: &mut state,
            theme: Theme::default(),
            viewport,
            time: UiTime::default(),
            locale: None,
            text_measurer: None,
        });
        let mut seen = 0usize;
        let (_response, ()) = ui.virtual_list("items", 1000, 20.0, 100.0, |ui, range| {
            seen = range.len();
            for i in range {
                let _ = ui.list_row(format!("item {i}"));
            }
        });
        ui.end();
        assert!(seen > 0 && seen < 20, "seen={seen}");
    }

    #[test]
    fn drag_payload_roundtrip_type() {
        let payload = DragPayload::new(42u32);
        assert!(payload.is::<u32>());
        assert!(!payload.is::<i32>());
        assert_eq!(payload.downcast_ref::<u32>(), Some(&42));
    }

    #[test]
    fn button_registers_access_node() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let input = Input::default();
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut ui = Ui::new(UiBuilder {
            input: &input,
            draw: &mut draw,
            state: &mut state,
            theme: Theme::default(),
            viewport,
            time: UiTime::default(),
            locale: None,
            text_measurer: None,
        });
        let response = ui.button("开始");
        assert!(ui
            .state()
            .access
            .by_id(response.id)
            .is_some_and(|n| n.role == Role::Button && n.label == "开始"));
        ui.end();
    }

    #[test]
    fn tabs_and_collapsible_change_state() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut selected = 0usize;
        let mut open = false;
        {
            let mut input = Input::default();
            // second tab roughly at x > half of width
            input.on_cursor(220.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let tabs = ui.tabs(&["一", "二"], &mut selected);
            assert!(tabs.hovered || tabs.changed || selected <= 1);
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_cursor(220.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            input.begin_frame();
            input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let _ = ui.tabs(&["一", "二"], &mut selected);
            ui.end();
        }
        assert_eq!(selected, 1);

        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let (response, body) = ui.collapsible("高级", &mut open, |ui| {
                ui.label("内容");
            });
            assert!(body.is_none());
            ui.end();
            let _ = response;
        }
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            input.begin_frame();
            input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let (_response, body) = ui.collapsible("高级", &mut open, |ui| {
                ui.label("内容");
            });
            assert!(open);
            assert!(body.is_some());
            ui.end();
        }
    }

    #[test]
    fn modal_body_runs_when_open() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let input = Input::default();
        let viewport = Rect::new(0.0, 0.0, 640.0, 480.0);
        {
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            ui.open_modal("confirm", "确认删除");
            let ran = ui
                .modal("confirm", "确认删除", |ui| {
                    ui.label("不可撤销");
                    7u32
                })
                .map(|(_, v)| v);
            assert_eq!(ran, Some(7));
            assert!(ui.state().overlays.modal_body_drawn);
            ui.end();
        }
    }

    #[test]
    fn modal_traps_tab_focus() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 640.0, 480.0);
        let input = Input::default();
        let mut confirm = WidgetId::NONE;
        {
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let _outside = ui.button("外面");
            ui.open_modal("confirm", "确认");
            let _ = ui.modal("confirm", "确认", |ui| {
                confirm = ui.button("确认").id;
            });
            ui.end();
        }
        assert_eq!(state.focused, Some(confirm));
        {
            let mut input = Input::default();
            input.on_key(Key::Tab, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let _outside = ui.button("外面");
            ui.open_modal("confirm", "确认");
            let _ = ui.modal("confirm", "确认", |ui| {
                confirm = ui.button("确认").id;
            });
            ui.end();
        }
        assert_eq!(state.focused, Some(confirm));
    }

    #[test]
    fn focused_button_clicks_on_enter() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let id;
        {
            let input = Input::default();
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            id = ui.button("开始").id;
            ui.request_focus(id);
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_key(Key::Enter, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let response = ui.button("开始");
            assert!(response.clicked);
            assert_eq!(response.id, id);
            ui.end();
        }
    }

    #[test]
    fn grid_places_cells_left_to_right() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let input = Input::default();
        let viewport = Rect::new(0.0, 0.0, 300.0, 200.0);
        let mut ui = Ui::new(UiBuilder {
            input: &input,
            draw: &mut draw,
            state: &mut state,
            theme: Theme::default(),
            viewport,
            time: UiTime::default(),
            locale: None,
            text_measurer: None,
        });
        let mut rects = Vec::new();
        ui.grid(3, 2, 40.0, |ui| {
            for i in 0..6 {
                let r = ui.button(format!("{i}")).rect;
                rects.push(r);
            }
        });
        ui.end();
        assert_eq!(rects.len(), 6);
        assert!(rects[1].x > rects[0].x);
        assert!((rects[3].y - rects[0].y).abs() > 30.0);
        assert!((rects[0].y - rects[1].y).abs() < 1.0);
    }

    #[test]
    fn prefs_scale_applied_on_ui_new() {
        let mut state = UiState::new();
        state.prefs = UiPrefs::default().scale(2.0);
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let input = Input::default();
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let base = Theme::default().metrics.button_height;
        let ui = Ui::new(UiBuilder {
            input: &input,
            draw: &mut draw,
            state: &mut state,
            theme: Theme::default(),
            viewport,
            time: UiTime::default(),
            locale: None,
            text_measurer: None,
        });
        assert!((ui.theme().metrics.button_height - base * 2.0).abs() < 1e-3);
        ui.end();
    }

    #[test]
    fn combo_box_selects_option() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 320.0, 400.0);
        let mut selected = 0usize;
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let _ = ui.combo_box("res", &["低", "中", "高"], &mut selected);
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            input.begin_frame();
            input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let _ = ui.combo_box("res", &["低", "中", "高"], &mut selected);
            ui.end();
        }
        assert!(state.memory.values().any(|m| m.opened));
        {
            let mut input = Input::default();
            // second option under header (~ button_height + row_height*1.5)
            input.on_cursor(40.0, 36.0 + 28.0 + 14.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let _ = ui.combo_box("res", &["低", "中", "高"], &mut selected);
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 36.0 + 28.0 + 14.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            input.begin_frame();
            input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let response = ui.combo_box("res", &["低", "中", "高"], &mut selected);
            ui.end();
            assert!(response.changed || selected == 1, "selected={selected}");
        }
        assert_eq!(selected, 1);
    }

    #[test]
    fn scroll_area_scrolls_focused_widget_into_view() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 200.0, 200.0);
        let target;
        let scroll_id;
        {
            let input = Input::default();
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let (response, last) = ui.scroll_area("bag", 100.0, |ui| {
                let mut last = WidgetId::NONE;
                for i in 0..20 {
                    last = ui.button(format!("item{i}")).id;
                }
                last
            });
            scroll_id = response.id;
            target = last;
            ui.request_focus(target);
            ui.scroll_into_view(target);
            ui.end();
        }
        let scroll = state.memory(scroll_id).map(|m| m.scroll.y).unwrap_or(0.0);
        assert!(scroll > 0.0, "scroll={scroll}");
    }

    #[test]
    fn number_field_steps_with_arrow_keys() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut value = 1.0f32;
        let id;
        {
            let input = Input::default();
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            id = ui.number_field(&mut value, 0.0..=10.0, 0.5).id;
            ui.request_focus(id);
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_key(Key::Up, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let _ = ui.number_field(&mut value, 0.0..=10.0, 0.5);
            ui.end();
        }
        assert!((value - 1.5).abs() < 1e-4, "value={value}");
    }

    #[test]
    fn key_binding_field_captures_pressed_key() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut binding = None;
        let id;
        {
            let input = Input::default();
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            id = ui.key_binding_field(&mut binding).id;
            ui.request_focus(id);
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_key(Key::F, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let response = ui.key_binding_field(&mut binding);
            assert!(response.changed);
            ui.end();
        }
        assert_eq!(binding, Some(Key::F));
    }

    #[test]
    fn popup_menu_selects_item() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut open = true;
        let anchor = Rect::new(10.0, 10.0, 80.0, 24.0);
        let row_h = Theme::default().metrics.row_height;
        let y = anchor.y + anchor.h + 2.0 + row_h * 1.5;
        let x = anchor.x + 10.0;
        {
            let mut input = Input::default();
            input.on_cursor(x, y);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let _ = ui.popup_menu("ctx", anchor, &mut open, &["复制", "粘贴", "删除"]);
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_cursor(x, y);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            input.begin_frame();
            input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let chosen = ui.popup_menu("ctx", anchor, &mut open, &["复制", "粘贴", "删除"]);
            ui.end();
            assert_eq!(chosen, Some(1));
        }
        assert!(!open);
    }

    #[test]
    fn text_area_inserts_newline_on_enter() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut value = String::from("ab");
        let id;
        {
            let input = Input::default();
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            id = ui.text_area(&mut value, 3).id;
            ui.request_focus(id);
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_key(Key::Enter, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let response = ui.text_area(&mut value, 3);
            assert!(response.changed);
            ui.end();
        }
        assert_eq!(value, "ab\n");
    }

    #[test]
    fn split_pane_updates_ratio_when_dragged() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 400.0, 240.0);
        let mut ratio = 0.5f32;
        {
            let mut input = Input::default();
            input.on_cursor(200.0, 40.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let _ = ui.split_pane(
                "main",
                &mut ratio,
                180.0,
                |ui| {
                    ui.label("left");
                },
                |ui| {
                    ui.label("right");
                },
            );
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_cursor(120.0, 40.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let response = ui.split_pane(
                "main",
                &mut ratio,
                180.0,
                |ui| {
                    ui.label("left");
                },
                |ui| {
                    ui.label("right");
                },
            );
            assert!(response.changed || (ratio - 0.5).abs() > 0.05);
            ui.end();
        }
        assert!(ratio < 0.45, "ratio={ratio}");
    }

    #[test]
    fn slot_click_reports_response() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        {
            let mut input = Input::default();
            input.on_cursor(20.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let _ = ui.slot(0u32, 32.0, None, Some(5), false);
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_cursor(20.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            input.begin_frame();
            input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let response = ui.slot(0u32, 32.0, None, Some(5), true);
            assert!(response.clicked);
            ui.end();
        }
    }

    #[test]
    fn tree_expands_and_reports_click() {
        use std::collections::HashSet;
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 320.0, 400.0);
        let nodes = vec![TreeNode {
            id: 1,
            label: "root".into(),
            children: vec![TreeNode {
                id: 2,
                label: "child".into(),
                children: vec![],
            }],
        }];
        let mut expanded = HashSet::new();
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let _ = ui.tree("inv", &nodes, &mut expanded);
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            input.begin_frame();
            input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime::default(),
                locale: None,
                text_measurer: None,
            });
            let clicked = ui.tree("inv", &nodes, &mut expanded);
            assert_eq!(clicked, Some(1));
            ui.end();
        }
        assert!(expanded.contains(&1));
    }

    #[test]
    fn health_bar_and_key_prompt_allocate() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let input = Input::default();
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut ui = Ui::new(UiBuilder {
            input: &input,
            draw: &mut draw,
            state: &mut state,
            theme: Theme::default(),
            viewport,
            time: UiTime::default(),
            locale: None,
            text_measurer: None,
        });
        let bar = ui.health_bar(50.0, 100.0, Color::rgb(0.8, 0.2, 0.2));
        let prompt = ui.key_prompt(Key::E, "交互");
        assert!(bar.rect.h > 0.0);
        assert!(prompt.rect.w > 0.0);
        ui.end();
    }

    #[test]
    fn stack_overlays_children_at_same_origin() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let input = Input::default();
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut ui = Ui::new(UiBuilder {
            input: &input,
            draw: &mut draw,
            state: &mut state,
            theme: Theme::default(),
            viewport,
            time: UiTime::default(),
            locale: None,
            text_measurer: None,
        });
        let mut a = Rect::default();
        let mut b = Rect::default();
        ui.stack(80.0, |ui| {
            a = ui.button("A").rect;
            b = ui.button("B").rect;
        });
        ui.end();
        assert!((a.x - b.x).abs() < 1e-3);
        assert!((a.y - b.y).abs() < 1e-3);
    }

    #[test]
    fn table_lays_out_header_and_rows() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let input = Input::default();
        let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ui = Ui::new(UiBuilder {
            input: &input,
            draw: &mut draw,
            state: &mut state,
            theme: Theme::default(),
            viewport,
            time: UiTime::default(),
            locale: None,
            text_measurer: None,
        });
        let mut cells = 0usize;
        let response = ui.table("inv", &["名", "数"], 3, 28.0, |ui, row, col| {
            cells += 1;
            ui.label(format!("{row}:{col}"));
        });
        ui.end();
        assert_eq!(cells, 6);
        assert!(response.rect.h > 28.0 * 3.0);
    }

    #[test]
    fn scroll_area_x_moves_with_wheel() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 200.0, 120.0);
        let mut input = Input::default();
        input.on_cursor(40.0, 40.0);
        input.on_wheel(-2.0);
        let mut ui = Ui::new(UiBuilder {
            input: &input,
            draw: &mut draw,
            state: &mut state,
            theme: Theme::default(),
            viewport,
            time: UiTime::default(),
            locale: None,
            text_measurer: None,
        });
        let (response, _) = ui.scroll_area_x("strip", 80.0, |ui| {
            for i in 0..20 {
                let _ = ui.button(format!("c{i}"));
            }
        });
        let id = response.id;
        ui.end();
        let scroll = state.memory(id).map(|m| m.scroll.x).unwrap_or(0.0);
        assert!(scroll > 0.0, "scroll_x={scroll}");
    }

    #[test]
    fn rich_text_draws_spans() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let input = Input::default();
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut ui = Ui::new(UiBuilder {
            input: &input,
            draw: &mut draw,
            state: &mut state,
            theme: Theme::default(),
            viewport,
            time: UiTime::default(),
            locale: None,
            text_measurer: None,
        });
        let response = ui.rich_text(|t| {
            t.span("HP ");
            t.span("100").color(Color::rgb(1.0, 0.3, 0.3)).strong();
        });
        assert!(response.rect.w > 0.0);
        ui.end();
    }

    #[test]
    fn interact_reports_double_click() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        // first click
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime {
                    dt: 1.0 / 60.0,
                    seconds: 1.0,
                },
                locale: None,
                text_measurer: None,
            });
            let _ = ui.button("点");
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            input.begin_frame();
            input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime {
                    dt: 1.0 / 60.0,
                    seconds: 1.05,
                },
                locale: None,
                text_measurer: None,
            });
            let _ = ui.button("点");
            ui.end();
        }
        // second click soon after
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime {
                    dt: 1.0 / 60.0,
                    seconds: 1.2,
                },
                locale: None,
                text_measurer: None,
            });
            let _ = ui.button("点");
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            input.begin_frame();
            input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime {
                    dt: 1.0 / 60.0,
                    seconds: 1.25,
                },
                locale: None,
                text_measurer: None,
            });
            let response = ui.button("点");
            assert!(response.double_clicked, "expected double click");
            ui.end();
        }
    }

    #[test]
    fn interact_reports_long_press() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime {
                    dt: 1.0 / 60.0,
                    seconds: 2.0,
                },
                locale: None,
                text_measurer: None,
            });
            let _ = ui.button("按住");
            ui.end();
        }
        {
            let mut input = Input::default();
            input.on_cursor(40.0, 20.0);
            input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
            // 模拟按住：边沿已过，仅保留 down。
            input.begin_frame();
            let mut ui = Ui::new(UiBuilder {
                input: &input,
                draw: &mut draw,
                state: &mut state,
                theme: Theme::default(),
                viewport,
                time: UiTime {
                    dt: 1.0 / 60.0,
                    seconds: 2.6,
                },
                locale: None,
                text_measurer: None,
            });
            let response = ui.button("按住");
            assert!(response.long_pressed);
            ui.end();
        }
    }

    #[test]
    fn minimap_viewport_allocates() {
        let mut state = UiState::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        let input = Input::default();
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut ui = Ui::new(UiBuilder {
            input: &input,
            draw: &mut draw,
            state: &mut state,
            theme: Theme::default(),
            viewport,
            time: UiTime::default(),
            locale: None,
            text_measurer: None,
        });
        let response = ui.minimap_viewport(
            "map",
            spark_core::Vec2::new(96.0, 96.0),
            Rect::new(0.0, 0.0, 1000.0, 1000.0),
            Rect::new(100.0, 200.0, 200.0, 120.0),
        );
        assert!((response.rect.w - 96.0).abs() < 1.0);
        ui.end();
    }
}

