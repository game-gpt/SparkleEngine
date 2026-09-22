//! 样式属性过渡占位。
//!
//! 描述「哪个视觉属性、多久、何种缓动」；由 [`MotionManager`](super::MotionManager)
//! 在伪态变化时套用到 opacity / scale 轨道。

use super::Easing;

/// 可参与过渡的视觉属性键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleProperty {
    /// 不透明度。
    Opacity,
    /// 相对中心的缩放。
    Scale,
    /// 平移（预留）。
    Translation,
    /// 颜色（预留）。
    Color,
}

/// 单条属性过渡描述：属性 + 时长 + 缓动。
#[derive(Debug, Clone, Copy)]
pub struct Transition {
    /// 作用的样式属性。
    pub property: StyleProperty,
    /// 过渡时长（秒）。
    pub duration: f32,
    /// 时间 → 权重的缓动曲线。
    pub easing: Easing,
}
