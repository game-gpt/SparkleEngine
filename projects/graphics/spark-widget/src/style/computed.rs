//! 局部样式与计算结果。

use spark_core::Color;

use super::theme::Theme;

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub opacity: Option<f32>,
    pub corner_radius: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub background: Color,
    pub foreground: Color,
    pub opacity: f32,
    pub corner_radius: f32,
}

impl ComputedStyle {
    pub fn resolve(theme: &Theme, local: &Style) -> Self {
        Self {
            background: local.background.unwrap_or(theme.colors.surface),
            foreground: local.foreground.unwrap_or(theme.colors.foreground),
            opacity: local.opacity.unwrap_or(1.0),
            corner_radius: local.corner_radius.unwrap_or(0.0),
        }
    }
}
