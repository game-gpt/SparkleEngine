//! UI 属性动效（非 spark-animator）。

mod easing;
mod spring;
mod transition;

pub use easing::Easing;
pub use spring::SpringConfig;
pub use transition::Transition;

use crate::id::WidgetId;

#[derive(Debug, Default)]
pub struct MotionManager {
    // 占位：后续接 Tween / Spring 求值表。
    _active: Vec<WidgetId>,
}

impl MotionManager {
    pub fn tick(&mut self, _dt: f32) {
        // TODO: 推进过渡并写回 ComputedStyle / transform。
    }
}
