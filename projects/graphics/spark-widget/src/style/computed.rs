//! 局部样式与按伪态解析。

use spark_types::Color;

use crate::node::{WidgetKind, WidgetNode, WidgetStateFlags};

use super::theme::{ButtonTreatment, Theme};

/// 节点上可选覆盖的局部样式声明（未设则走主题）。
#[derive(Debug, Clone, Default)]
pub struct Style {
    /// 背景色覆盖。
    pub background: Option<Color>,
    /// 前景 / 文字色覆盖。
    pub foreground: Option<Color>,
    /// 不透明度覆盖（`0..=1`）。
    pub opacity: Option<f32>,
    /// 圆角半径覆盖。
    pub corner_radius: Option<f32>,
    /// 覆盖主题字号（Label / Button 文本）。
    pub font_size: Option<f32>,
    /// 覆盖强调色（ProgressBar 填充等）。
    pub accent: Option<Color>,
    /// 边框色覆盖。
    pub border_color: Option<Color>,
    /// 边框宽度覆盖；`0` 表示不绘制边框。
    pub border_width: Option<f32>,
}

/// 解析后的最终绘制样式（主题 + kind 默认 + 局部 + 伪态）。
#[derive(Debug, Clone)]
pub struct ComputedStyle {
    /// 背景色。
    pub background: Color,
    /// 前景 / 文字色。
    pub foreground: Color,
    /// 边框色。
    pub border: Color,
    /// 边框宽度。
    pub border_width: f32,
    /// 强调色。
    pub accent: Color,
    /// 不透明度。
    pub opacity: f32,
    /// 圆角半径。
    pub corner_radius: f32,
    /// 字号。
    pub font_size: f32,
}

impl ComputedStyle {
    /// 无伪态时的基础解析（测试与简单用途）。
    pub fn resolve(theme: &Theme, local: &Style) -> Self {
        Self {
            background: local.background.unwrap_or(theme.colors.surface),
            foreground: local.foreground.unwrap_or(theme.colors.foreground),
            border: theme.colors.border,
            border_width: local.border_width.unwrap_or(theme.metrics.border_width),
            accent: theme.colors.accent,
            opacity: local.opacity.unwrap_or(1.0),
            corner_radius: local.corner_radius.unwrap_or(theme.metrics.corner_radius),
            font_size: local.font_size.unwrap_or(theme.typography.body_size),
        }
    }

    /// 按 Widget 种类与交互态解析最终样式。
    pub fn resolve_for(theme: &Theme, node: &WidgetNode) -> Self {
        let mut style = Self::resolve(theme, &node.style);
        apply_kind_defaults(theme, node.kind, &mut style);
        // 局部字段覆盖 kind 默认，但必须在伪态之前，否则 hover / pressed 会被盖掉。
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
        if let Some(accent) = node.style.accent {
            style.accent = accent;
        }
        if let Some(border) = node.style.border_color {
            style.border = border;
        }
        if let Some(width) = node.style.border_width {
            style.border_width = width.max(0.0);
        }
        apply_pseudo(theme, &node.state, node.kind, &mut style);
        style
    }
}

fn apply_kind_defaults(theme: &Theme, kind: WidgetKind, style: &mut ComputedStyle) {
    match kind {
        WidgetKind::Button => match theme.button_treatment {
            ButtonTreatment::Accent => {
                style.background = theme.colors.accent;
                style.foreground = Color::rgb(0.05, 0.06, 0.08);
                style.corner_radius = 6.0;
            }
            ButtonTreatment::Quiet => {
                style.background = theme.colors.control;
                style.foreground = theme.colors.foreground;
                style.corner_radius = theme.metrics.corner_radius;
            }
        },
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
            // 容器默认透明，避免全屏 column 用 surface 盖住下层装饰（标题远景等）。
            style.background = Color::rgba(0.0, 0.0, 0.0, 0.0);
        }
        _ => {}
    }
}

fn apply_pseudo(theme: &Theme, state: &WidgetStateFlags, kind: WidgetKind, style: &mut ComputedStyle) {
    if state.disabled {
        style.background = theme.colors.disabled;
        style.foreground = Color::rgb(0.75, 0.76, 0.78);
        style.opacity *= 0.7;
        return;
    }

    if state.selected {
        style.background = theme.colors.selection;
        style.foreground = theme.colors.foreground;
    }

    match kind {
        WidgetKind::Button => {
            // 透明底文字按钮：idle 保持声明字色，hover/focus/pressed 走主题菜单色。
            let text_btn = style.background.a < 0.01;
            if text_btn {
                if state.pressed {
                    style.foreground = theme.menu_item.pressed;
                }
                else if state.hovered || state.focused {
                    style.foreground = theme.menu_item.hover;
                }
            }
            else if state.pressed {
                style.background = match theme.button_treatment {
                    ButtonTreatment::Accent => multiply_rgb(theme.colors.accent, 0.75),
                    ButtonTreatment::Quiet if state.selected => multiply_rgb(theme.colors.selection, 0.8),
                    ButtonTreatment::Quiet => multiply_rgb(theme.colors.control, 0.8),
                };
            }
            else if state.hovered {
                style.background = match theme.button_treatment {
                    ButtonTreatment::Accent => multiply_rgb(theme.colors.accent, 1.12),
                    ButtonTreatment::Quiet if state.selected => multiply_rgb(theme.colors.selection, 1.12),
                    ButtonTreatment::Quiet => theme.colors.control_hover,
                };
            }
            if state.focused {
                style.border = theme.colors.focus;
            }
        }
        WidgetKind::TextField | WidgetKind::TextArea => {
            if state.focused {
                style.border = theme.colors.accent;
            }
            else if state.hovered {
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

    // 校验错误优先于焦点环，避免聚焦后掩盖错误状态。
    if state.invalid {
        style.border = theme.colors.danger;
        style.border_width = style.border_width.max(theme.metrics.border_width);
    }
}

fn multiply_rgb(color: Color, factor: f32) -> Color {
    Color::rgba((color.r * factor).min(1.0), (color.g * factor).min(1.0), (color.b * factor).min(1.0), color.a)
}
