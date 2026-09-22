//! 主题 tokens。

use spark_types::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub colors: UiColors,
    pub typography: Typography,
    pub spacing: Spacing,
}

impl Default for Theme {
    fn default() -> Self {
        Self { colors: UiColors::default(), typography: Typography::default(), spacing: Spacing::default() }
    }
}

#[derive(Debug, Clone)]
pub struct UiColors {
    pub background: Color,
    pub surface: Color,
    pub foreground: Color,
    pub accent: Color,
    pub danger: Color,
    pub disabled: Color,
    pub border: Color,
    pub focus: Color,
    pub track: Color,
}

impl Default for UiColors {
    fn default() -> Self {
        Self {
            background: Color::rgb(0.08, 0.09, 0.11),
            surface: Color::rgb(0.14, 0.16, 0.20),
            foreground: Color::rgb(0.92, 0.94, 0.96),
            accent: Color::rgb(0.35, 0.65, 0.95),
            danger: Color::rgb(0.90, 0.30, 0.28),
            disabled: Color::rgb(0.45, 0.47, 0.50),
            border: Color::rgb(0.28, 0.32, 0.38),
            focus: Color::rgb(0.95, 0.85, 0.35),
            track: Color::rgb(0.22, 0.24, 0.28),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Typography {
    pub body_size: f32,
    pub heading_size: f32,
    pub label_size: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Self { body_size: 16.0, heading_size: 22.0, label_size: 14.0 }
    }
}

#[derive(Debug, Clone)]
pub struct Spacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self { xs: 4.0, sm: 8.0, md: 12.0, lg: 20.0 }
    }
}
