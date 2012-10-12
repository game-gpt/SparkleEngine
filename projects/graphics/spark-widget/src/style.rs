//! 主题与语义样式。控件读主题，不硬编码颜色。

use spark_core::Color;

use crate::motion::{Easing, MotionSpec};

/// 文本语气。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextTone {
    #[default]
    Primary,
    Muted,
    Danger,
    Success,
    Warning,
}

/// 按钮变体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Danger,
    Ghost,
}

/// 交互视觉状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InteractState {
    #[default]
    Normal,
    Hovered,
    Pressed,
    Focused,
    Disabled,
    Selected,
}

#[derive(Debug, Clone, Copy)]
pub struct UiColors {
    pub background: Color,
    pub panel: Color,
    pub panel_title: Color,
    pub border: Color,
    pub text: Color,
    pub text_muted: Color,
    pub text_danger: Color,
    pub text_success: Color,
    pub text_warning: Color,
    pub primary: Color,
    pub primary_hover: Color,
    pub primary_pressed: Color,
    pub danger: Color,
    pub danger_hover: Color,
    pub focus_ring: Color,
    pub track: Color,
    pub knob: Color,
    pub checkbox_on: Color,
}

impl Default for UiColors {
    fn default() -> Self {
        Self {
            background: Color::rgb(0.05, 0.06, 0.10),
            panel: Color::rgb(0.10, 0.12, 0.18),
            panel_title: Color::rgb(0.14, 0.18, 0.28),
            border: Color::rgb(0.45, 0.70, 0.95),
            text: Color::rgb(0.92, 0.96, 1.0),
            text_muted: Color::rgb(0.65, 0.70, 0.78),
            text_danger: Color::rgb(0.95, 0.45, 0.45),
            text_success: Color::rgb(0.45, 0.85, 0.55),
            text_warning: Color::rgb(0.95, 0.80, 0.40),
            primary: Color::rgb(0.12, 0.22, 0.38),
            primary_hover: Color::rgb(0.18, 0.35, 0.58),
            primary_pressed: Color::rgb(0.20, 0.45, 0.75),
            danger: Color::rgb(0.40, 0.12, 0.14),
            danger_hover: Color::rgb(0.55, 0.18, 0.20),
            focus_ring: Color::rgb(0.55, 0.85, 1.0),
            track: Color::rgb(0.12, 0.16, 0.24),
            knob: Color::rgb(0.45, 0.75, 0.95),
            checkbox_on: Color::rgb(0.35, 0.75, 0.55),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Typography {
    pub label: f32,
    pub button: f32,
    pub heading: f32,
    pub small: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            label: 18.0,
            button: 22.0,
            heading: 28.0,
            small: 14.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Spacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            xs: 4.0,
            sm: 8.0,
            md: 16.0,
            lg: 24.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WidgetMetrics {
    pub button_height: f32,
    pub row_height: f32,
    pub checkbox: f32,
    pub slider_height: f32,
}

impl Default for WidgetMetrics {
    fn default() -> Self {
        Self {
            button_height: 36.0,
            row_height: 28.0,
            checkbox: 22.0,
            slider_height: 24.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MotionTheme {
    pub hover: MotionSpec,
    pub press: MotionSpec,
    pub panel: MotionSpec,
    pub reduced_motion: bool,
}

impl Default for MotionTheme {
    fn default() -> Self {
        Self {
            hover: MotionSpec::ms(90, Easing::EaseOut),
            press: MotionSpec::ms(60, Easing::EaseOut),
            panel: MotionSpec::ms(120, Easing::EaseInOut),
            reduced_motion: false,
        }
    }
}

/// 全局主题。游戏可替换整份或局部覆盖颜色。
#[derive(Debug, Clone, Copy, Default)]
pub struct Theme {
    pub colors: UiColors,
    pub typography: Typography,
    pub spacing: Spacing,
    pub metrics: WidgetMetrics,
    pub motion: MotionTheme,
}

impl Theme {
    pub fn text_color(&self, tone: TextTone) -> Color {
        match tone {
            TextTone::Primary => self.colors.text,
            TextTone::Muted => self.colors.text_muted,
            TextTone::Danger => self.colors.text_danger,
            TextTone::Success => self.colors.text_success,
            TextTone::Warning => self.colors.text_warning,
        }
    }

    pub fn button_fill(&self, variant: ButtonVariant, state: InteractState) -> Color {
        match variant {
            ButtonVariant::Danger => match state {
                InteractState::Pressed => self.colors.danger_hover,
                InteractState::Hovered | InteractState::Focused => self.colors.danger_hover,
                InteractState::Disabled => Color::rgb(0.20, 0.12, 0.12),
                _ => self.colors.danger,
            },
            ButtonVariant::Ghost => match state {
                InteractState::Pressed => Color::rgba(1.0, 1.0, 1.0, 0.12),
                InteractState::Hovered | InteractState::Focused => Color::rgba(1.0, 1.0, 1.0, 0.08),
                InteractState::Disabled => Color::rgba(1.0, 1.0, 1.0, 0.02),
                _ => Color::rgba(0.0, 0.0, 0.0, 0.0),
            },
            ButtonVariant::Secondary => match state {
                InteractState::Pressed => self.colors.primary_pressed,
                InteractState::Hovered | InteractState::Focused => self.colors.primary_hover,
                InteractState::Disabled => Color::rgb(0.10, 0.12, 0.16),
                _ => self.colors.panel,
            },
            ButtonVariant::Primary => match state {
                InteractState::Pressed => self.colors.primary_pressed,
                InteractState::Hovered | InteractState::Focused => self.colors.primary_hover,
                InteractState::Disabled => Color::rgb(0.10, 0.12, 0.16),
                _ => self.colors.primary,
            },
        }
    }
}
