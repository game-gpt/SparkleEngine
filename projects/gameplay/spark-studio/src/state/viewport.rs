//! 场景视口相机（平移 / 缩放，非 ECS 权威）。

/// 场景视口 2D 相机状态。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportState {
    /// 屏幕空间平移 X（像素）。
    pub pan_x: f32,
    /// 屏幕空间平移 Y（像素）。
    pub pan_y: f32,
    /// 世界单位 → 屏幕像素缩放。
    pub zoom: f32,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self { pan_x: 0.0, pan_y: 0.0, zoom: 1.0 }
    }
}
