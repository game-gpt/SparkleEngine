//! 状态之间的过渡。条件全部成立才开始。

/// 一条过渡上的条件。多个条件是与关系。
#[derive(Debug, Clone, PartialEq)]
pub enum AnimatorCondition {
    BoolEquals { parameter: String, value: bool },
    FloatGreater { parameter: String, value: f32 },
    Trigger { parameter: String },
}

/// 从 `from` 到 `to`。`duration` 为 0 时立即切换。
#[derive(Debug, Clone, PartialEq)]
pub struct AnimatorTransition {
    pub from: String,
    pub to: String,
    pub duration: f32,
    pub conditions: Vec<AnimatorCondition>,
}

impl AnimatorTransition {
    pub fn new(from: impl Into<String>, to: impl Into<String>, duration: f32) -> Self {
        Self { from: from.into(), to: to.into(), duration: duration.max(0.0), conditions: Vec::new() }
    }

    pub fn when(mut self, condition: AnimatorCondition) -> Self {
        self.conditions.push(condition);
        self
    }
}
