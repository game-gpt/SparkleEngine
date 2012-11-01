//! 数值与按键绑定控件。

use spark_core::{Color, Rect};
use spark_input::Key;

use crate::accessibility::{AccessNode, Role};
use crate::response::Response;
use crate::ui::Ui;

impl Ui<'_> {
    /// 数值框：点击聚焦后可键入，上下键按 `step` 调节，失焦时写回。
    pub fn number_field(
        &mut self,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
        step: f32,
    ) -> Response {
        let min = *range.start();
        let max = (*range.end()).max(min);
        let step = if step.abs() < 1e-12 { 1.0 } else { step.abs() };
        let height = self.theme.metrics.button_height;
        let rect = self.allocate(height, None);
        let id = self.id_from("number_field");
        self.state.register_focusable(id, rect);
        let mut response = self.interact(id, rect, true);
        if self.keyboard_activate(id) {
            response.clicked = true;
        }

        let was_focused = self.state.memory(id).map(|m| m.opened).unwrap_or(false);
        let focused = response.focused;
        if focused && !was_focused {
            let mem = self.state.memory_mut(id);
            mem.opened = true;
            mem.edit_buf = format_number(*value);
            mem.cursor = mem.edit_buf.chars().count();
        }
        if !focused && was_focused {
            let buf = self.state.memory(id).map(|m| m.edit_buf.clone()).unwrap_or_default();
            if let Ok(parsed) = buf.parse::<f32>() {
                let nv = parsed.clamp(min, max);
                if (nv - *value).abs() > 1e-6 {
                    *value = nv;
                    response.changed = true;
                }
            }
            self.state.memory_mut(id).opened = false;
            self.state.memory_mut(id).edit_buf.clear();
        }

        if focused {
            if self.input.key_pressed(Key::Up) {
                let nv = (*value + step).clamp(min, max);
                if (nv - *value).abs() > 1e-6 {
                    *value = nv;
                    response.changed = true;
                }
                let mem = self.state.memory_mut(id);
                mem.edit_buf = format_number(*value);
                mem.cursor = mem.edit_buf.chars().count();
            }
            if self.input.key_pressed(Key::Down) {
                let nv = (*value - step).clamp(min, max);
                if (nv - *value).abs() > 1e-6 {
                    *value = nv;
                    response.changed = true;
                }
                let mem = self.state.memory_mut(id);
                mem.edit_buf = format_number(*value);
                mem.cursor = mem.edit_buf.chars().count();
            }

            let mut buf = self.state.memory(id).map(|m| m.edit_buf.clone()).unwrap_or_default();
            let mut cursor = self.state.memory(id).map(|m| m.cursor).unwrap_or(buf.chars().count());
            cursor = cursor.min(buf.chars().count());
            if !self.input.text().is_empty() {
                for ch in self.input.text().chars() {
                    if !(ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == '+') {
                        continue;
                    }
                    let insert_at = buf
                        .char_indices()
                        .nth(cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(buf.len());
                    buf.insert(insert_at, ch);
                    cursor += 1;
                    response.changed = true;
                }
            }
            if self.input.key_pressed(Key::Backspace) && cursor > 0 {
                let remove_at = buf
                    .char_indices()
                    .nth(cursor - 1)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                buf.remove(remove_at);
                cursor -= 1;
                response.changed = true;
            }
            if self.input.key_pressed(Key::Left) {
                cursor = cursor.saturating_sub(1);
            }
            if self.input.key_pressed(Key::Right) {
                cursor = (cursor + 1).min(buf.chars().count());
            }
            if self.input.key_pressed(Key::Enter) {
                if let Ok(parsed) = buf.parse::<f32>() {
                    let nv = parsed.clamp(min, max);
                    if (nv - *value).abs() > 1e-6 {
                        *value = nv;
                        response.changed = true;
                    }
                    buf = format_number(*value);
                    cursor = buf.chars().count();
                }
            }
            let mem = self.state.memory_mut(id);
            mem.edit_buf = buf;
            mem.cursor = cursor;
            mem.opened = true;
        }

        let display = if focused {
            self.state
                .memory(id)
                .map(|m| m.edit_buf.clone())
                .unwrap_or_else(|| format_number(*value))
        } else {
            format_number(*value)
        };

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
            &display,
        );
        if focused {
            let cursor = self.state.memory(id).map(|m| m.cursor).unwrap_or(0);
            let prefix: String = display.chars().take(cursor).collect();
            let cx = rect.x + pad + self.measure_width(&prefix, size);
            let blink = ((self.time.seconds * 2.0) as i64) % 2 == 0;
            if blink {
                self.draw.fill_rect(
                    Rect::new(cx, rect.y + 6.0, 2.0, rect.h - 12.0),
                    self.theme.colors.text,
                );
            }
        }

        self.access(
            AccessNode::new(id, Role::TextField)
                .label("number")
                .value(display)
                .rect(rect)
                .focusable(true)
                .focused(focused),
        );
        response
    }

    /// 按键绑定：聚焦后按下任意键写入 `binding`。
    pub fn key_binding_field(&mut self, binding: &mut Option<Key>) -> Response {
        let height = self.theme.metrics.button_height;
        let rect = self.allocate(height, None);
        let id = self.id_from("key_binding");
        self.state.register_focusable(id, rect);
        let mut response = self.interact(id, rect, true);
        if self.keyboard_activate(id) {
            response.clicked = true;
        }
        let focused = response.focused;
        if focused {
            // Escape 清除绑定；其它键写入。避免吞掉 Tab 以免破坏焦点导航。
            if self.input.key_pressed(Key::Escape) {
                if binding.is_some() {
                    *binding = None;
                    response.changed = true;
                }
            } else if let Some(key) = self
                .input
                .keys_pressed()
                .find(|k| !matches!(k, Key::Tab | Key::Escape))
            {
                if *binding != Some(key) {
                    *binding = Some(key);
                    response.changed = true;
                }
            }
        }

        let label = binding
            .map(key_label)
            .unwrap_or_else(|| {
                if focused {
                    "按下按键…".into()
                } else {
                    "未绑定".into()
                }
            });
        let fill = if focused {
            self.theme.colors.primary_hover
        } else if response.hovered {
            self.theme.colors.primary
        } else {
            self.theme.colors.panel
        };
        self.draw.fill_rect(rect, fill);
        self.draw.fill_rect(
            Rect::new(rect.x - 1.0, rect.y - 1.0, rect.w + 2.0, rect.h + 2.0),
            if focused {
                self.theme.colors.focus_ring
            } else {
                self.theme.colors.border
            },
        );
        let size = self.theme.typography.label;
        self.draw.text(
            rect.x + 8.0,
            rect.y + (rect.h - size) * 0.5,
            size,
            self.theme.colors.text,
            &label,
        );
        self.access(
            AccessNode::new(id, Role::Button)
                .label("key binding")
                .value(label)
                .rect(rect)
                .focusable(true)
                .focused(focused),
        );
        response
    }
}

fn format_number(value: f32) -> String {
    if value.fract().abs() < 1e-4 {
        format!("{:.0}", value)
    } else {
        format!("{:.3}", value)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn key_label(key: Key) -> String {
    match key {
        Key::Escape => "Esc".into(),
        Key::Enter => "Enter".into(),
        Key::Space => "Space".into(),
        Key::Tab => "Tab".into(),
        Key::Backspace => "Backspace".into(),
        Key::Left => "Left".into(),
        Key::Right => "Right".into(),
        Key::Up => "Up".into(),
        Key::Down => "Down".into(),
        Key::LShift => "LShift".into(),
        Key::RShift => "RShift".into(),
        other => format!("{other:?}"),
    }
}
