//! 文本测量、排版与基础编辑。

mod edit;

pub use edit::{apply_text_input, TextEditAction};

use spark_core::{Color, Vec2};
use spark_font::GlyphCache;

/// 文本样式。
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

/// 文本布局结果。
#[derive(Debug, Clone, Default)]
pub struct TextLayout {
    pub size: Vec2,
    pub baseline: f32,
    pub line_count: usize,
}

/// 可替换的文本测量器。
pub trait TextMeasurer: Send {
    fn measure(&mut self, text: &str, style: &TextStyle, max_width: Option<f32>) -> TextLayout;
}

/// 按字符数粗估（无字体时的回退）。
#[derive(Debug, Default, Clone, Copy)]
pub struct EstimateMeasurer;

impl TextMeasurer for EstimateMeasurer {
    fn measure(&mut self, text: &str, style: &TextStyle, max_width: Option<f32>) -> TextLayout {
        measure_estimate(text, style, max_width)
    }
}

/// 基于 `spark-font::GlyphCache` 的测量。
pub struct FontMeasurer {
    cache: GlyphCache,
}

impl FontMeasurer {
    pub fn new(cache: GlyphCache) -> Self {
        Self { cache }
    }

    pub fn try_system() -> Option<Self> {
        GlyphCache::load_system().ok().map(Self::new)
    }

    pub fn cache_mut(&mut self) -> &mut GlyphCache {
        &mut self.cache
    }
}

impl TextMeasurer for FontMeasurer {
    fn measure(&mut self, text: &str, style: &TextStyle, max_width: Option<f32>) -> TextLayout {
        let px = style.size.max(1.0);
        if max_width.is_none() {
            let w = self.cache.measure(text, px);
            return TextLayout {
                size: Vec2::new(w, px * style.line_height),
                baseline: px,
                line_count: 1,
            };
        }
        // 简单按字符折行（后续接完整 shaping）。
        let max_w = max_width.unwrap().max(0.0);
        let mut line_w = 0.0_f32;
        let mut max_line = 0.0_f32;
        let mut lines = 1_usize;
        for ch in text.chars() {
            let adv = self
                .cache
                .glyph(ch, px)
                .map(|g| g.advance)
                .unwrap_or(px * 0.5);
            if line_w + adv > max_w && line_w > 0.0 {
                max_line = max_line.max(line_w);
                line_w = adv;
                lines += 1;
            } else {
                line_w += adv;
            }
        }
        max_line = max_line.max(line_w);
        TextLayout {
            size: Vec2::new(max_line.min(max_w.max(max_line)), px * style.line_height * lines as f32),
            baseline: px,
            line_count: lines,
        }
    }
}

/// 兼容旧调用：始终走估算。
pub fn measure_plain(text: &str, style: &TextStyle, max_width: Option<f32>) -> TextLayout {
    measure_estimate(text, style, max_width)
}

fn measure_estimate(text: &str, style: &TextStyle, max_width: Option<f32>) -> TextLayout {
    let char_w = style.size * 0.55;
    if let Some(max_w) = max_width {
        if max_w > 0.0 && !text.is_empty() {
            let per_line = ((max_w / char_w).floor() as usize).max(1);
            let chars = text.chars().count();
            let lines = chars.div_ceil(per_line).max(1);
            return TextLayout {
                size: Vec2::new(max_w.min(chars as f32 * char_w), style.size * style.line_height * lines as f32),
                baseline: style.size,
                line_count: lines,
            };
        }
    }
    let w = text.chars().count() as f32 * char_w;
    TextLayout {
        size: Vec2::new(w, style.size * style.line_height),
        baseline: style.size,
        line_count: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_wraps_when_max_width_set() {
        let style = TextStyle {
            size: 10.0,
            ..TextStyle::default()
        };
        let layout = measure_plain("abcdefghij", &style, Some(30.0));
        assert!(layout.line_count >= 2);
    }
}
