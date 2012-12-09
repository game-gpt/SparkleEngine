//! 局部样式与按伪态解析。

use spark_core::Color;

use crate::node::{WidgetKind, WidgetNode, WidgetStateFlags};

use super::theme::Theme;

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub opacity: Option<f32>,
    pub corner_radius: Option<f32>,
    /// 覆盖主题字号（Label / Button 文本）。
    pub font_size: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub background: Color,
    pub foreground: Color,
    pub border: Color,
    pub accent: Color,
    pub opacity: f32,
    pub corner_radius: f32,
    pub font_size: f32,
}

impl ComputedStyle {
    /// 无伪态时的基础解析（测试与简单用途）。
    pub fn resolve(theme: &Theme, local: &Style) -> Self {
        Self {
            background: local.background.unwrap_or(theme.colors.surface),
            foreground: local.foreground.unwrap_or(theme.colors.foreground),
            border: theme.colors.border,
            accent: theme.colors.accent,
            opacity: local.opacity.unwrap_or(1.0),
            corner_radius: local.corner_radius.unwrap_or(4.0),
            font_size: local.font_size.unwrap_or(theme.typography.body_size),
        }
    }

    /// 按 Widget 种类与交互态解析最终样式。
    pub fn resolve_for(theme: &Theme, node: &WidgetNode) -> Self {
        let mut style = Self::resolve(theme, &node.style);
        apply_kind_defaults(theme, node.kind, &mut style);
        apply_pseudo(theme, &node.state, node.kind, &mut style);
        if let Some(bg) = node.style.background {
            style.background = bg;
        }
        if let Some(fg) = node.style.foreground {
            style.foreground = fg;
        }
        if let Some(op) = node.style.opacity {
            style.opacity = op;
        }
        if let Some(r) = node.style.corner_radius {
            style.corner_radius = r;
        }
        if let Some(fs) = node.style.font_size {
            style.font_size = fs;
        }
        style
    }
}

fn apply_kind_defaults(theme: &Theme, kind: WidgetKind, style: &mut ComputedStyle) {
    match kind {
        WidgetKind::Button => {
            style.background = theme.colors.accent;
            style.foreground = Color::rgb(0.05, 0.06, 0.08);
            style.corner_radius = 6.0;
        }
        WidgetKind::Label => {
            style.background = Color::rgba(0.0, 0.0, 0.0, 0.0);
        }
        WidgetKind::Checkbox | WidgetKind::Toggle | WidgetKind::Radio => {
            style.background = Color::rgba(0.0, 0.0, 0.0, 0.0);
        }
        WidgetKind::TextField | WidgetKind::TextArea => {
            style.background = theme.colors.background;
            style.border = theme.colors.border;
        }
        WidgetKind::Slider | WidgetKind::ProgressBar => {
            style.background = theme.colors.background;
        }
        WidgetKind::Separator => {
            style.background = theme.colors.border;
        }
        WidgetKind::Modal | WidgetKind::Popup | WidgetKind::Panel => {
            style.background = theme.colors.surface;
        }
        WidgetKind::Container | WidgetKind::Root | WidgetKind::Spacer => {
            if style.background.a <= 0.0 {
                style.background = Color::rgba(0.0, 0.0, 0.0, 0.0);
            }
        }
        _ => {}
    }
}

fn apply_pseudo(
    theme: &Theme,
    state: &WidgetStateFlags,
    kind: WidgetKind,
    style: &mut ComputedStyle,
) {
    if state.disabled {
        style.background = theme.colors.disabled;
        style.foreground = Color::rgb(0.75, 0.76, 0.78);
        style.opacity *= 0.7;
        return;
    }

    match kind {
        WidgetKind::Button => {
            if state.pressed {
                style.background = multiply_rgb(theme.colors.accent, 0.75);
            } else if state.hovered {
                style.background = multiply_rgb(theme.colors.accent, 1.12);
            }
            if state.focused {
                style.border = theme.colors.focus;
            }
        }
        WidgetKind::TextField | WidgetKind::TextArea => {
            if state.focused {
                style.border = theme.colors.accent;
            } else if state.hovered {
                style.border = multiply_rgb(theme.colors.border, 1.3);
            }
        }
        WidgetKind::Checkbox | WidgetKind::Toggle | WidgetKind::Radio => {
            if state.checked {
                style.accent = theme.colors.accent;
            }
            if state.focused {
                style.border = theme.colors.focus;
            }
        }
        _ => {
            if state.hovered && style.background.a > 0.01 {
                style.background = multiply_rgb(style.background, 1.08);
            }
            if state.focused {
                style.border = theme.colors.focus;
            }
        }
    }
}

fn multiply_rgb(color: Color, factor: f32) -> Color {
    Color::rgba(
        (color.r * factor).min(1.0),
        (color.g * factor).min(1.0),
        (color.b * factor).min(1.0),
        color.a,
    )
}
