//! 富文本片段构建与绘制。

use spark_core::{Color, Rect};

use crate::accessibility::{AccessNode, Role};
use crate::response::Response;
use crate::style::TextTone;
use crate::ui::Ui;

/// 一段富文本。
#[derive(Debug, Clone)]
pub struct RichSpan {
    pub text: String,
    pub color: Option<Color>,
    pub strong: bool,
}

/// 富文本构建器（立即模式闭包内使用）。
#[derive(Debug, Default)]
pub struct RichText {
    spans: Vec<RichSpan>,
}

impl RichText {
    pub fn span(&mut self, text: impl Into<String>) -> &mut RichSpan {
        self.spans.push(RichSpan {
            text: text.into(),
            color: None,
            strong: false,
        });
        self.spans.last_mut().unwrap()
    }

    pub fn spans(&self) -> &[RichSpan] {
        &self.spans
    }
}

impl RichSpan {
    pub fn color(&mut self, color: Color) -> &mut Self {
        self.color = Some(color);
        self
    }

    pub fn strong(&mut self) -> &mut Self {
        self.strong = true;
        self
    }

    pub fn tone(&mut self, tone: TextTone, theme_color: Color) -> &mut Self {
        let _ = tone;
        self.color = Some(theme_color);
        self
    }
}

impl Ui<'_> {
    /// 富文本行：多段颜色 / 强调，单行水平排布。
    pub fn rich_text(&mut self, build: impl FnOnce(&mut RichText)) -> Response {
        let mut rich = RichText::default();
        build(&mut rich);
        let base = self.theme.typography.label;
        let height = base * 1.35 + 4.0;
        let rect = self.allocate(height, None);
        let id = self.id_from("rich_text");
        let mut x = rect.x;
        let mut plain = String::new();
        for span in &rich.spans {
            let size = if span.strong { base * 1.15 } else { base };
            let color = span.color.unwrap_or(self.theme.colors.text);
            self.draw.text(
                x,
                rect.y + (rect.h - size) * 0.5,
                size,
                color,
                &span.text,
            );
            x += self.measure_width(&span.text, size);
            plain.push_str(&span.text);
        }
        self.access(
            AccessNode::new(id, Role::Label)
                .label(plain)
                .rect(Rect::new(rect.x, rect.y, (x - rect.x).max(1.0), rect.h)),
        );
        Response::empty(id, rect)
    }
}
