//! 布局度量：DPI、UI 缩放与安全区。

use crate::layout::Insets;

/// 每帧布局度量（物理像素坐标系下解释）。
#[derive(Debug, Clone, Copy)]
pub struct UiMetrics {
    /// 窗口 DPI 缩放（如 1.0 / 1.5 / 2.0）。
    pub dpi_scale: f32,
    /// 用户 UI 缩放（叠加在 DPI 之上）。
    pub ui_scale: f32,
    /// 安全区（刘海 / 圆角等），单位与 `screen_size` 相同。
    pub safe_area: Insets,
}

impl Default for UiMetrics {
    fn default() -> Self {
        Self { dpi_scale: 1.0, ui_scale: 1.0, safe_area: Insets::default() }
    }
}

impl UiMetrics {
    pub fn new(dpi_scale: f32) -> Self {
        Self { dpi_scale: dpi_scale.max(0.01), ..Self::default() }
    }

    /// 文本与固有控件尺寸使用的综合缩放。
    pub fn content_scale(&self) -> f32 {
        (self.dpi_scale * self.ui_scale).max(0.01)
    }
}
