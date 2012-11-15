//! 文本测量、排版与编辑（占位）。

use spark_core::{Color, Vec2};

/// 文本样式占位。
#[derive(Debug, Clone)]
pub struct TextStyle {
    pub size: f32,
    pub color: Color,
    pub line_height: f32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            size: 16.0,
            color: Color::rgb(0.92, 0.94, 0.96),
            line_height: 1.25,
        }
    }
}

/// 文本布局结果占位。
#[derive(Debug, Clone, Default)]
pub struct TextLayout {
    pub size: Vec2,
    pub baseline: f32,
    pub line_count: usize,
}

/// 测量纯文本（占位：按字符数粗估，后续接 spark-font）。
pub fn measure_plain(text: &str, style: &TextStyle, max_width: Option<f32>) -> TextLayout {
    let _ = max_width;
    let w = text.chars().count() as f32 * style.size * 0.55;
    TextLayout {
        size: Vec2::new(w, style.size * style.line_height),
        baseline: style.size,
        line_count: 1,
    }
}
