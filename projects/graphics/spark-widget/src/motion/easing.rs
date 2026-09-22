//! 缓动曲线。
//!
//! 将归一化时间 `t ∈ [0,1]` 映射为插值权重，供 [`Transition`](super::Transition)
//! 与 [`MotionManager`](super::MotionManager) 使用。

/// 常用缓动曲线种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Easing {
    /// 匀速线性：`f(t) = t`。
    #[default]
    Linear,
    /// 先慢后快（二次入）。
    EaseIn,
    /// 先快后慢（二次出）。
    EaseOut,
    /// 两端慢、中间快。
    EaseInOut,
}

impl Easing {
    /// 采样缓动权重；输入会钳制到 `[0, 1]`。
    pub fn sample(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                }
                else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
        }
    }
}
