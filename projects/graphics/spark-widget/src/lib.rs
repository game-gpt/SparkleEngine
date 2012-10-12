//! 立即模式 Widget 系统。
//!
//! 推荐入口是 [`Ui`]：统一 ID、状态、命中、主题与布局。
//! 底部的 `panel` / `label` / `button` 等自由函数仍可用，但新页面应走 `Ui`。
//!
//! 界面控件叫 **Widget**，避免与 ECS `Component` 混淆。
//! UI 缓动在 [`motion`]，不属于 `spark-animator`。

mod id;
mod layout;
pub mod motion;
mod response;
mod state;
mod style;
mod text_source;
mod ui;

pub use id::WidgetId;
pub use layout::{Align, Direction, Insets, Justify, Layout, LayoutCursor, Size};
pub use motion::{
    Easing, MotionId, MotionProperty, MotionScheduler, MotionSequence, MotionSpec, MotionTick,
    MotionValue, SequenceStep, Spring, SpringParams, Transition, Tween,
};
pub use response::Response;
pub use state::{FocusSource, UiState, WidgetMemory};
pub use style::{
    ButtonVariant, InteractState, MotionTheme, Spacing, TextTone, Theme, Typography, UiColors,
    WidgetMetrics,
};
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
            });
            ui.scope("bag", |ui| {
                second = ui.id_from(7u32);
            });
            ui.end();
        }
        assert_eq!(first, second);
        assert!(!first.is_none());
    }
}

