//! UI 动效子系统。
//!
//! 控制 Widget 属性如何响应交互，不是角色动画，也不是世界特效。
//!
//! - [`Tween`]：数学插值器，实现细节
//! - [`Transition`]：单个属性从旧值到新值
//! - [`MotionScheduler`]：统一推进，由 UI 帧在布局前调用
//!
//! 与 `spark-animator` 无关：那边管 clip / 状态机 / ECS 实体。

mod easing;
mod scheduler;
mod sequence;
mod spring;
mod transition;
mod tween;

pub use easing::Easing;
pub use scheduler::{MotionId, MotionScheduler, MotionTick};
pub use sequence::{MotionSequence, SequenceStep};
pub use spring::{Spring, SpringParams};
pub use transition::{MotionProperty, MotionSpec, MotionValue, Transition};
pub use tween::Tween;
