//! 弹簧配置占位。
//!
//! 预留给物理弹簧插值；当前 [`MotionManager`](super::MotionManager) 仍走时长缓动。

/// 弹簧物理参数（刚度 / 阻尼 / 质量）。
#[derive(Debug, Clone, Copy)]
pub struct SpringConfig {
    /// 弹簧刚度。
    pub stiffness: f32,
    /// 阻尼系数。
    pub damping: f32,
    /// 质量。
    pub mass: f32,
}

impl SpringConfig {
    /// 偏「利落」的默认弹簧（较高刚度、中等阻尼）。
    pub fn snappy() -> Self {
        Self { stiffness: 300.0, damping: 20.0, mass: 1.0 }
    }
}

impl Default for SpringConfig {
    fn default() -> Self {
        Self::snappy()
    }
}
