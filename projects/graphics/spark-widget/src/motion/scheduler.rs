//! 统一推进所有活动过渡。调用方跨帧持有，不绑 ECS。

use std::collections::HashMap;

use crate::motion::sequence::MotionSequence;
use crate::motion::transition::{MotionProperty, MotionSpec, MotionValue, Transition};

/// 跨帧识别一个控件。由 UI 层分配，与 [`crate::FocusId`] 可共用数值空间。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MotionId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SlotKey {
    id: MotionId,
    property: MotionProperty,
}

/// 本帧推进后的摘要。
#[derive(Debug, Clone, Default)]
pub struct MotionTick {
    /// 是否有布局相关属性变化。
    pub layout_dirty: bool,
    /// 本帧结束的槽位数。
    pub finished: usize,
}

/// UI 动效调度器。生命周期由 Widget 树或即时模式宿主持有。
#[derive(Debug, Default, Clone)]
pub struct MotionScheduler {
    slots: HashMap<SlotKey, Transition>,
    sequences: HashMap<MotionId, MotionSequence>,
    /// 每个属性最近一次采样值，过渡结束后仍可读。
    values: HashMap<SlotKey, MotionValue>,
}

impl MotionScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// 开始或改写一条属性过渡。同键会覆盖。
    pub fn transition(
        &mut self,
        id: MotionId,
        property: MotionProperty,
        from: MotionValue,
        to: MotionValue,
        spec: MotionSpec,
    ) -> bool {
        let Some(transition) = Transition::start(property, from, to, spec) else {
            return false;
        };
        let key = SlotKey { id, property };
        self.values.insert(key, transition.sample());
        self.slots.insert(key, transition);
        true
    }

    /// 若已有同属性过渡则改目标，否则新建。
    pub fn transition_to(
        &mut self,
        id: MotionId,
        property: MotionProperty,
        from: MotionValue,
        to: MotionValue,
        spec: MotionSpec,
    ) -> bool {
        let key = SlotKey { id, property };
        if let Some(existing) = self.slots.get_mut(&key) {
            existing.retarget(to);
            self.values.insert(key, existing.sample());
            return true;
        }
        self.transition(id, property, from, to, spec)
    }

    pub fn play_sequence(&mut self, id: MotionId, sequence: MotionSequence) {
        if let Some(property) = sequence.property() {
            if let Some(value) = sequence.sample() {
                self.values.insert(SlotKey { id, property }, value);
            }
        }
        self.sequences.insert(id, sequence);
    }

    pub fn cancel(&mut self, id: MotionId, property: MotionProperty) {
        self.slots.remove(&SlotKey { id, property });
    }

    pub fn cancel_all(&mut self, id: MotionId) {
        self.slots.retain(|key, _| key.id != id);
        self.sequences.remove(&id);
        self.values.retain(|key, _| key.id != id);
    }

    pub fn value(&self, id: MotionId, property: MotionProperty) -> Option<MotionValue> {
        self.values.get(&SlotKey { id, property }).copied()
    }

    pub fn float(&self, id: MotionId, property: MotionProperty) -> Option<f32> {
        self.value(id, property).and_then(MotionValue::as_float)
    }

    pub fn is_active(&self, id: MotionId, property: MotionProperty) -> bool {
        self.slots.contains_key(&SlotKey { id, property })
            || self
                .sequences
                .get(&id)
                .is_some_and(|sequence| sequence.property() == Some(property))
    }

    /// 推进所有活动过渡。应在交互状态更新之后、布局与绘制之前调用。
    pub fn tick(&mut self, dt: f32) -> MotionTick {
        let mut report = MotionTick::default();
        let keys: Vec<SlotKey> = self.slots.keys().copied().collect();
        for key in keys {
            let Some(transition) = self.slots.get_mut(&key) else {
                continue;
            };
            let finished = transition.tick(dt);
            let sample = transition.sample();
            if key.property.affects_layout() {
                report.layout_dirty = true;
            }
            self.values.insert(key, sample);
            if finished {
                self.slots.remove(&key);
                report.finished += 1;
            }
        }

        let sequence_ids: Vec<MotionId> = self.sequences.keys().copied().collect();
        for id in sequence_ids {
            let Some(sequence) = self.sequences.get_mut(&id) else {
                continue;
            };
            if let Some(property) = sequence.property() {
                if property.affects_layout() {
                    report.layout_dirty = true;
                }
                if let Some(value) = sequence.sample() {
                    self.values.insert(SlotKey { id, property }, value);
                }
            }
            let finished = sequence.tick(dt);
            if let Some(property) = sequence.property() {
                if let Some(value) = sequence.sample() {
                    self.values.insert(SlotKey { id, property }, value);
                }
            }
            if finished {
                self.sequences.remove(&id);
                report.finished += 1;
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion::easing::Easing;

    #[test]
    fn opacity_transition_does_not_dirty_layout() {
        let mut motion = MotionScheduler::new();
        let id = MotionId(1);
        assert!(motion.transition(
            id,
            MotionProperty::Opacity,
            MotionValue::Float(0.0),
            MotionValue::Float(1.0),
            MotionSpec::ms(100, Easing::Linear),
        ));
        let report = motion.tick(0.05);
        assert!(!report.layout_dirty);
        let v = motion.float(id, MotionProperty::Opacity).unwrap();
        assert!((v - 0.5).abs() < 1e-3);
        let report = motion.tick(0.05);
        assert_eq!(report.finished, 1);
        assert!((motion.float(id, MotionProperty::Opacity).unwrap() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn width_transition_marks_layout_dirty() {
        let mut motion = MotionScheduler::new();
        let id = MotionId(2);
        motion.transition(
            id,
            MotionProperty::Width,
            MotionValue::Float(10.0),
            MotionValue::Float(20.0),
            MotionSpec::ease_out(0.1),
        );
        let report = motion.tick(0.016);
        assert!(report.layout_dirty);
    }

    #[test]
    fn retarget_keeps_current_sample_as_new_from() {
        let mut motion = MotionScheduler::new();
        let id = MotionId(3);
        motion.transition(
            id,
            MotionProperty::Scale,
            MotionValue::Float(1.0),
            MotionValue::Float(1.2),
            MotionSpec::linear(0.2),
        );
        motion.tick(0.1);
        let mid = motion.float(id, MotionProperty::Scale).unwrap();
        assert!((mid - 1.1).abs() < 1e-3);
        motion.transition_to(
            id,
            MotionProperty::Scale,
            MotionValue::Float(mid),
            MotionValue::Float(1.0),
            MotionSpec::linear(0.2),
        );
        motion.tick(0.0);
        assert!((motion.float(id, MotionProperty::Scale).unwrap() - mid).abs() < 1e-3);
    }
}
