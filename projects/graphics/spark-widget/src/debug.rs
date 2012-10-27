//! UI 调试叠加：边界、hot / active / focused、ID 冲突。

use spark_core::{Color, Rect};
use spark_renderer::DrawList;

use crate::id::WidgetId;
use crate::state::UiState;

/// 调试开关。由游戏或 `spark-debugger` 会话持有。
#[derive(Debug, Clone, Copy)]
pub struct UiDebug {
    pub enabled: bool,
    pub show_bounds: bool,
    pub show_padding: bool,
    pub show_clip: bool,
    pub show_ids: bool,
}

impl Default for UiDebug {
    fn default() -> Self {
        Self {
            enabled: false,
            show_bounds: true,
            show_padding: false,
            show_clip: true,
            show_ids: false,
        }
    }
}

impl UiDebug {
    pub fn on() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }

    /// 把本帧记忆中的矩形与交互态画到 `draw`。
    pub fn paint(&self, state: &UiState, draw: &mut DrawList) {
        if !self.enabled {
            return;
        }
        for (id, mem) in &state.memory {
            let Some(rect) = mem.last_rect else {
                continue;
            };
            if self.show_bounds {
                let color = highlight_color(state, *id);
                outline(draw, rect, color);
            }
            if self.show_ids {
                draw.text(
                    rect.x + 2.0,
                    rect.y + 2.0,
                    12.0,
                    Color::rgb(0.9, 0.95, 0.4),
                    &format!("{:x}", id.raw()),
                );
            }
        }
        if let Some(id) = state.hot {
            if let Some(rect) = state.memory(id).and_then(|m| m.last_rect) {
                outline(draw, rect, Color::rgb(0.2, 0.95, 0.4));
            }
        }
        if let Some(id) = state.active {
            if let Some(rect) = state.memory(id).and_then(|m| m.last_rect) {
                outline(draw, rect, Color::rgb(0.95, 0.55, 0.2));
            }
        }
        if let Some(id) = state.focused {
            if let Some(rect) = state.memory(id).and_then(|m| m.last_rect) {
                outline(draw, rect, Color::rgb(0.4, 0.75, 1.0));
            }
        }
        if self.show_ids {
            for id in state.id_conflicts() {
                if let Some(rect) = state.memory(id).and_then(|m| m.last_rect) {
                    draw.fill_rect(
                        Rect::new(rect.x, rect.y, rect.w.min(8.0), rect.h.min(8.0)),
                        Color::rgb(1.0, 0.2, 0.2),
                    );
                }
            }
        }
    }
}

fn highlight_color(state: &UiState, id: WidgetId) -> Color {
    if state.focused == Some(id) {
        Color::rgba(0.4, 0.75, 1.0, 0.9)
    } else if state.active == Some(id) {
        Color::rgba(0.95, 0.55, 0.2, 0.9)
    } else if state.hot == Some(id) {
        Color::rgba(0.2, 0.95, 0.4, 0.9)
    } else {
        Color::rgba(0.6, 0.6, 0.7, 0.45)
    }
}

fn outline(draw: &mut DrawList, rect: Rect, color: Color) {
    let t = 1.0;
    draw.fill_rect(Rect::new(rect.x, rect.y, rect.w, t), color);
    draw.fill_rect(Rect::new(rect.x, rect.y + rect.h - t, rect.w, t), color);
    draw.fill_rect(Rect::new(rect.x, rect.y, t, rect.h), color);
    draw.fill_rect(Rect::new(rect.x + rect.w - t, rect.y, t, rect.h), color);
}
