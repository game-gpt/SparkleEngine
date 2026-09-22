//! UI 属性动效（非 spark-animator）。

mod easing;
mod spring;
mod transition;

pub use easing::Easing;
pub use spring::SpringConfig;
pub use transition::{StyleProperty, Transition};

use std::collections::HashMap;

use crate::{
    id::WidgetId,
    node::{WidgetKind, WidgetNode},
    tree::WidgetTree,
};

#[derive(Debug, Clone, Copy)]
struct Track {
    from: f32,
    to: f32,
    elapsed: f32,
    duration: f32,
    easing: Easing,
    value: f32,
}

impl Track {
    fn set_target(&mut self, target: f32, duration: f32, easing: Easing) {
        if (self.to - target).abs() < 0.0001 && (self.value - target).abs() < 0.0001 {
            return;
        }
        self.from = self.value;
        self.to = target;
        self.elapsed = 0.0;
        self.duration = duration.max(0.001);
        self.easing = easing;
    }

    fn tick(&mut self, dt: f32) {
        self.elapsed = (self.elapsed + dt).min(self.duration);
        let t = self.easing.sample(self.elapsed / self.duration);
        self.value = self.from + (self.to - self.from) * t;
    }
}

#[derive(Debug, Clone, Copy)]
struct WidgetMotion {
    opacity: Track,
    scale: Track,
}

impl Default for WidgetMotion {
    fn default() -> Self {
        Self {
            opacity: Track { from: 1.0, to: 1.0, elapsed: 0.0, duration: 0.001, easing: Easing::EaseOut, value: 1.0 },
            scale: Track { from: 1.0, to: 1.0, elapsed: 0.0, duration: 0.001, easing: Easing::EaseOut, value: 1.0 },
        }
    }
}

/// 某控件当前动效采样（paint 时乘到 computed style）。
#[derive(Debug, Clone, Copy)]
pub struct MotionSample {
    /// 当前不透明度乘数（1 = 不变）。
    pub opacity: f32,
    /// 当前缩放乘数（相对节点中心，1 = 不变）。
    pub scale: f32,
}

impl Default for MotionSample {
    fn default() -> Self {
        Self { opacity: 1.0, scale: 1.0 }
    }
}

/// 按控件维护 opacity / scale 插值轨道，并在伪态变化时换目标。
#[derive(Debug)]
pub struct MotionManager {
    widgets: HashMap<WidgetId, WidgetMotion>,
    /// hover / press 等引起的不透明度过渡参数。
    pub opacity_transition: Transition,
    /// hover / press 等引起的缩放过渡参数。
    pub scale_transition: Transition,
}

impl Default for MotionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MotionManager {
    /// 使用默认时长缓动创建空管理器。
    pub fn new() -> Self {
        Self {
            widgets: HashMap::new(),
            opacity_transition: Transition { property: StyleProperty::Opacity, duration: 0.12, easing: Easing::EaseOut },
            scale_transition: Transition { property: StyleProperty::Scale, duration: 0.09, easing: Easing::EaseOut },
        }
    }

    /// 按伪态刷新目标，并推进插值。
    pub fn sync_and_tick(&mut self, tree: &WidgetTree, dt: f32) {
        let ids = tree.ids();
        for id in ids {
            let Some(node) = tree.node(id)
            else {
                continue;
            };
            if !should_animate(node) {
                continue;
            }
            let (opacity, scale) = targets_for(node);
            let entry = self.widgets.entry(id).or_default();
            entry.opacity.set_target(opacity, self.opacity_transition.duration, self.opacity_transition.easing);
            entry.scale.set_target(scale, self.scale_transition.duration, self.scale_transition.easing);
            entry.opacity.tick(dt);
            entry.scale.tick(dt);
        }
        self.widgets.retain(|id, _| tree.node(*id).is_some());
    }

    /// 仅推进已有轨道（不根据树刷新目标）。
    pub fn tick(&mut self, dt: f32) {
        for motion in self.widgets.values_mut() {
            motion.opacity.tick(dt);
            motion.scale.tick(dt);
        }
    }

    /// 取某控件当前采样；无轨道时返回单位样本。
    pub fn sample(&self, id: WidgetId) -> MotionSample {
        self.widgets.get(&id).map(|m| MotionSample { opacity: m.opacity.value, scale: m.scale.value }).unwrap_or_default()
    }
}

fn should_animate(node: &WidgetNode) -> bool {
    matches!(
        node.kind,
        WidgetKind::Button
            | WidgetKind::Checkbox
            | WidgetKind::Toggle
            | WidgetKind::Radio
            | WidgetKind::Slider
            | WidgetKind::TextField
            | WidgetKind::Panel
            | WidgetKind::Popup
            | WidgetKind::Modal
            | WidgetKind::Toast
    )
}

fn targets_for(node: &WidgetNode) -> (f32, f32) {
    if node.state.disabled {
        return (0.55, 1.0);
    }
    let mut opacity: f32 = 1.0;
    let mut scale: f32 = 1.0;
    if node.state.pressed {
        opacity = 0.92;
        scale = 0.97;
    }
    else if node.state.hovered {
        opacity = 1.0;
        scale = 1.03;
    }
    if node.state.focused && !node.state.pressed {
        scale = scale.max(1.02);
    }
    (opacity, scale)
}
