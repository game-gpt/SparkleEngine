//! 浮层：tooltip、modal、toast。脱离布局，在 [`Ui::end`] 绘制。

use spark_core::{Color, Rect, Vec2};
use spark_input::{Key, MouseBtn};

use crate::id::WidgetId;
use crate::state::UiState;
use crate::style::Theme;

/// 本帧排队的浮层命令。
#[derive(Debug, Clone)]
pub(crate) enum OverlayCmd {
    Tooltip {
        anchor: Rect,
        text: String,
    },
}

#[derive(Debug, Default)]
pub struct OverlayState {
    pub(crate) queue: Vec<OverlayCmd>,
    /// 打开的 modal（跨帧）。
    pub open_modal: Option<WidgetId>,
    pub modal_title: String,
    /// toast 队列（跨帧）。
    pub toasts: Vec<ToastEntry>,
    /// 本帧是否阻断下层输入。
    pub(crate) block_input: bool,
}

#[derive(Debug, Clone)]
pub struct ToastEntry {
    pub text: String,
    pub age: f32,
    pub life: f32,
}

impl OverlayState {
    pub fn begin_frame(&mut self) {
        self.queue.clear();
        self.block_input = self.open_modal.is_some();
    }

    pub fn tooltip(&mut self, anchor: Rect, text: impl Into<String>) {
        self.queue.push(OverlayCmd::Tooltip {
            anchor,
            text: text.into(),
        });
    }

    pub fn open_modal(&mut self, id: WidgetId, title: impl Into<String>) {
        self.open_modal = Some(id);
        self.modal_title = title.into();
        self.block_input = true;
    }

    pub fn close_modal(&mut self) {
        self.open_modal = None;
        self.modal_title.clear();
        self.block_input = false;
    }

    pub fn toast(&mut self, text: impl Into<String>) {
        self.toasts.push(ToastEntry {
            text: text.into(),
            age: 0.0,
            life: 2.5,
        });
    }

    pub(crate) fn tick_toasts(&mut self, dt: f32) {
        for toast in &mut self.toasts {
            toast.age += dt;
        }
        self.toasts.retain(|t| t.age < t.life);
    }
}

/// 在帧末绘制浮层并处理 Escape / 点击外部关闭。
pub(crate) fn flush_overlays(
    state: &mut UiState,
    draw: &mut spark_renderer::DrawList,
    theme: &Theme,
    viewport: Rect,
    input: &spark_input::Input,
    dt: f32,
) {
    state.overlays.tick_toasts(dt);

    if state.overlays.open_modal.is_some() {
        if input.key_pressed(Key::Escape) {
            state.overlays.close_modal();
        }
    }

    // Modal 遮罩
    if let Some(modal_id) = state.overlays.open_modal {
        let dim = Color::rgba(0.0, 0.0, 0.0, 0.55);
        draw.fill_rect(viewport, dim);
        let w = (viewport.w * 0.5).clamp(280.0, 520.0);
        let h = 200.0;
        let panel = Rect::new(
            viewport.x + (viewport.w - w) * 0.5,
            viewport.y + (viewport.h - h) * 0.5,
            w,
            h,
        );
        draw.fill_rect(panel, theme.colors.panel);
        draw.fill_rect(
            Rect::new(panel.x, panel.y, panel.w, 32.0),
            theme.colors.panel_title,
        );
        draw.text(
            panel.x + 12.0,
            panel.y + 6.0,
            theme.typography.label,
            theme.colors.text,
            &state.overlays.modal_title,
        );
        // 点击遮罩外部关闭
        let (mx, my) = input.mouse_pos();
        let p = Vec2::new(mx, my);
        if input.mouse_pressed(MouseBtn::Left) && viewport.contains(p) && !panel.contains(p) {
            let _ = modal_id;
            state.overlays.close_modal();
        }
        state.memory_mut(modal_id).last_rect = Some(panel);
    }

    // Tooltip
        for cmd in state.overlays.queue.clone() {
            let OverlayCmd::Tooltip { anchor, text } = cmd;
            paint_tooltip(draw, theme, viewport, anchor, &text);
        }

    // Toast：底部居中堆叠
    let mut y = viewport.y + viewport.h - 48.0;
    for toast in state.overlays.toasts.iter().rev() {
        let fade = (1.0 - (toast.age / toast.life)).clamp(0.0, 1.0);
        let tw = (toast.text.chars().count() as f32 * theme.typography.label * 0.55 + 32.0)
            .min(viewport.w - 40.0);
        let th = 36.0;
        let rect = Rect::new(viewport.x + (viewport.w - tw) * 0.5, y - th, tw, th);
        let mut fill = theme.colors.primary_hover;
        fill.a *= fade;
        draw.fill_rect(rect, fill);
        let mut color = theme.colors.text;
        color.a *= fade;
        draw.text(
            rect.x + 12.0,
            rect.y + 8.0,
            theme.typography.label,
            color,
            &toast.text,
        );
        y -= th + 8.0;
    }
}

fn paint_tooltip(
    draw: &mut spark_renderer::DrawList,
    theme: &Theme,
    viewport: Rect,
    anchor: Rect,
    text: &str,
) {
    let pad = 8.0;
    let size = theme.typography.small;
    let tw = text.chars().count() as f32 * size * 0.55 + pad * 2.0;
    let th = size + pad * 2.0;
    let mut x = anchor.x;
    let mut y = anchor.y - th - 6.0;
    if y < viewport.y {
        y = anchor.y + anchor.h + 6.0;
    }
    if x + tw > viewport.x + viewport.w {
        x = viewport.x + viewport.w - tw;
    }
    x = x.max(viewport.x);
    let rect = Rect::new(x, y, tw, th);
    draw.fill_rect(
        Rect::new(rect.x - 1.0, rect.y - 1.0, rect.w + 2.0, rect.h + 2.0),
        theme.colors.border,
    );
    draw.fill_rect(rect, Color::rgb(0.08, 0.10, 0.16));
    draw.text(rect.x + pad, rect.y + pad, size, theme.colors.text, text);
}
