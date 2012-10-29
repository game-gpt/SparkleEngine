//! UI 使用偏好：缩放、减少动效、高对比。

use crate::style::{Theme, UiColors};

/// 跨会话偏好。存在 [`crate::UiState`] 上，由游戏写入。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiPrefs {
    /// 整体 UI 尺寸倍率。
    pub ui_scale: f32,
    /// 字号额外倍率（叠在 ui_scale 上）。
    pub font_scale: f32,
    /// 缩短或跳过过渡。
    pub reduced_motion: bool,
    /// 提高边框与文字对比。
    pub high_contrast: bool,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            ui_scale: 1.0,
            font_scale: 1.0,
            reduced_motion: false,
            high_contrast: false,
        }
    }
}

impl UiPrefs {
    pub fn scale(mut self, ui_scale: f32) -> Self {
        self.ui_scale = ui_scale.max(0.5);
        self
    }

    pub fn font_scale(mut self, font_scale: f32) -> Self {
        self.font_scale = font_scale.max(0.5);
        self
    }

    pub fn reduced_motion(mut self, on: bool) -> Self {
        self.reduced_motion = on;
        self
    }

    pub fn high_contrast(mut self, on: bool) -> Self {
        self.high_contrast = on;
        self
    }

    /// 按偏好调整主题副本，不改动原始主题资源。
    pub fn apply_theme(&self, mut theme: Theme) -> Theme {
        let s = self.ui_scale.max(0.5);
        let fs = (self.ui_scale * self.font_scale).max(0.5);
        theme.typography.label *= fs;
        theme.typography.button *= fs;
        theme.typography.heading *= fs;
        theme.typography.small *= fs;
        theme.spacing.xs *= s;
        theme.spacing.sm *= s;
        theme.spacing.md *= s;
        theme.spacing.lg *= s;
        theme.metrics.button_height *= s;
        theme.metrics.row_height *= s;
        theme.metrics.checkbox *= s;
        theme.metrics.slider_height *= s;
        theme.motion.reduced_motion = self.reduced_motion || theme.motion.reduced_motion;
        if self.high_contrast {
            theme.colors = high_contrast_colors(theme.colors);
        }
        theme
    }
}

fn high_contrast_colors(mut colors: UiColors) -> UiColors {
    colors.text = spark_core::Color::rgb(1.0, 1.0, 1.0);
    colors.text_muted = spark_core::Color::rgb(0.85, 0.85, 0.88);
    colors.border = spark_core::Color::rgb(0.95, 0.95, 1.0);
    colors.focus_ring = spark_core::Color::rgb(1.0, 1.0, 0.2);
    colors.background = spark_core::Color::rgb(0.0, 0.0, 0.0);
    colors.panel = spark_core::Color::rgb(0.05, 0.05, 0.08);
    colors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefs_scale_metrics() {
        let theme = Theme::default();
        let scaled = UiPrefs::default().scale(2.0).apply_theme(theme);
        assert!((scaled.metrics.button_height - theme.metrics.button_height * 2.0).abs() < 1e-3);
        assert!(scaled.typography.label > theme.typography.label);
    }
}
