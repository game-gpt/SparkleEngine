//! 顺序播放的过渡队列。同一时刻只跑当前一段。

use crate::motion::transition::{MotionProperty, MotionSpec, MotionValue, Transition};

/// 序列中的一步。
#[derive(Debug, Clone, PartialEq)]
pub struct SequenceStep {
    pub property: MotionProperty,
    pub from: MotionValue,
    pub to: MotionValue,
    pub spec: MotionSpec,
}

/// 按步骤推进的过渡序列。
#[derive(Debug, Clone, PartialEq)]
pub struct MotionSequence {
    steps: Vec<SequenceStep>,
    index: usize,
    current: Option<Transition>,
}

impl MotionSequence {
    pub fn new(steps: Vec<SequenceStep>) -> Self {
        let mut seq = Self {
            steps,
            index: 0,
            current: None,
        };
        seq.start_current();
        seq
    }

    pub fn finished(&self) -> bool {
        self.current.is_none() && self.index >= self.steps.len()
    }

    pub fn sample(&self) -> Option<MotionValue> {
        self.current.as_ref().map(Transition::sample)
    }

    pub fn property(&self) -> Option<MotionProperty> {
        self.current
            .as_ref()
            .map(|transition| transition.property)
            .or_else(|| self.steps.get(self.index).map(|step| step.property))
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        let Some(transition) = self.current.as_mut() else {
            return true;
        };
        if transition.tick(dt) {
            self.index += 1;
            self.start_current();
        }
        self.finished()
    }

    fn start_current(&mut self) {
        self.current = None;
        while self.index < self.steps.len() {
            let step = &self.steps[self.index];
            if let Some(transition) =
                Transition::start(step.property, step.from, step.to, step.spec)
            {
                self.current = Some(transition);
                return;
            }
            self.index += 1;
        }
    }
}
