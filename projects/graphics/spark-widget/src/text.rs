//! 文本测量、换行与单行输入框。
//!
//! 正式测量由调用方注入 [`TextMeasurer`]（通常包一层 `spark-font::GlyphCache`）。
//! 未注入时用估算宽度，仅作布局占位。

use spark_core::{Color, Rect};
use spark_input::Key;

use crate::response::Response;
use crate::ui::Ui;

/// 文本宽度测量。UI 不依赖具体字体 crate。
pub trait TextMeasurer {
    fn measure_width(&mut self, text: &str, size: f32) -> f32;
}

/// 未接字体时的估算器。中文按全角，ASCII 按 0.55em。
#[derive(Debug, Default, Clone, Copy)]
pub struct EstimateMeasurer;

impl TextMeasurer for EstimateMeasurer {
    fn measure_width(&mut self, text: &str, size: f32) -> f32 {
        estimate_text_width(text, size)
    }
}

pub(crate) fn estimate_text_width(text: &str, size: f32) -> f32 {
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

/// 按最大宽度换行。优先在空白处断行，否则按字符切。
pub fn wrap_lines(text: &str, max_width: f32, size: f32, measure: &mut dyn TextMeasurer) -> Vec<String> {
    if max_width <= 1.0 || text.is_empty() {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in split_keep_spaces(text) {
        let trial = if current.is_empty() {
            word.clone()
        } else {
            format!("{current}{word}")
        };
        if measure.measure_width(&trial, size) <= max_width {
            current = trial;
            continue;
        }
        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        if measure.measure_width(&word, size) <= max_width {
            current = word;
            continue;
        }
        for ch in word.chars() {
            let mut next = current.clone();
            next.push(ch);
            if measure.measure_width(&next, size) > max_width && !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current.push(ch);
            } else {
                current = next;
            }
        }
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

fn split_keep_spaces(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !buf.is_empty() {
                out.push(std::mem::take(&mut buf));
            }
            out.push(ch.to_string());
        } else {
            buf.push(ch);
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

impl Ui<'_> {
    pub(crate) fn measure_width(&mut self, text: &str, size: f32) -> f32 {
        if let Some(m) = self.text_measurer.as_deref_mut() {
            m.measure_width(text, size)
        } else {
            estimate_text_width(text, size)
        }
    }

    /// 可换行标签。
    pub fn label_wrapped(&mut self, text: impl AsRef<str>, max_width: Option<f32>) -> Response {
        let text = text.as_ref();
        let size = self.theme.typography.label;
        let width = max_width.unwrap_or_else(|| self.available_rect().w);
        let mut lines = {
            let mut estimate = EstimateMeasurer;
            let measurer: &mut dyn TextMeasurer = if let Some(m) = self.text_measurer.as_deref_mut() {
                m
            } else {
                &mut estimate
            };
            wrap_lines(text, width, size, measurer)
        };
        if lines.is_empty() {
            lines.push(String::new());
        }
        let line_h = size + 4.0;
        let height = line_h * lines.len() as f32;
        let rect = self.allocate(height, Some(width.min(self.available_rect().w)));
        let id = self.id_from(("label_wrap", text));
        let color = self.theme.colors.text;
        for (i, line) in lines.iter().enumerate() {
            self.draw.text(
                rect.x,
                rect.y + i as f32 * line_h + 2.0,
                size,
                color,
                line,
            );
        }
        Response::empty(id, rect)
    }

    /// 单行文本输入。值由调用方持有；光标写在 [`crate::WidgetMemory`]。
    pub fn text_field(&mut self, value: &mut String) -> Response {
        let height = self.theme.metrics.button_height;
        let rect = self.allocate(height, None);
        let id = self.id_from("text_field");
        self.state.register_focusable(id, rect);
        let mut response = self.interact(id, rect, true);
        let focused = response.focused;

        if focused {
            let mut cursor = self.state.memory(id).map(|m| m.cursor).unwrap_or(value.chars().count());
            cursor = cursor.min(value.chars().count());
            if !self.input.text().is_empty() {
                for ch in self.input.text().chars() {
                    if ch.is_control() {
                        continue;
                    }
                    let insert_at = value
                        .char_indices()
                        .nth(cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(value.len());
                    value.insert(insert_at, ch);
                    cursor += 1;
                    response.changed = true;
                }
            }
            if self.input.key_pressed(Key::Backspace) && cursor > 0 {
                let remove_at = value
                    .char_indices()
                    .nth(cursor - 1)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                value.remove(remove_at);
                cursor -= 1;
                response.changed = true;
            }
            if self.input.key_pressed(Key::Left) {
                cursor = cursor.saturating_sub(1);
            }
            if self.input.key_pressed(Key::Right) {
                cursor = (cursor + 1).min(value.chars().count());
            }
            self.state.memory_mut(id).cursor = cursor;
        }

        let fill = if focused {
            Color::rgb(0.10, 0.14, 0.22)
        } else {
            Color::rgb(0.08, 0.10, 0.16)
        };
        let border = if focused {
            self.theme.colors.focus_ring
        } else {
            self.theme.colors.border
        };
        self.draw.fill_rect(
            Rect::new(rect.x - 1.0, rect.y - 1.0, rect.w + 2.0, rect.h + 2.0),
            border,
        );
        self.draw.fill_rect(rect, fill);
        let size = self.theme.typography.label;
        let pad = 8.0;
        self.draw.text(
            rect.x + pad,
            rect.y + (rect.h - size) * 0.5,
            size,
            self.theme.colors.text,
            value.as_str(),
        );
        if focused {
            let cursor = self.state.memory(id).map(|m| m.cursor).unwrap_or(0);
            let prefix: String = value.chars().take(cursor).collect();
            let cx = rect.x + pad + self.measure_width(&prefix, size);
            let blink = ((self.time.seconds * 2.0) as i64) % 2 == 0;
            if blink {
                self.draw.fill_rect(
                    Rect::new(cx, rect.y + 6.0, 2.0, rect.h - 12.0),
                    self.theme.colors.text,
                );
            }
        }
        response
    }
}
