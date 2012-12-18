//! 状态机中的一个节点。它指向剪辑名，不持有剪辑数据。

#[derive(Debug, Clone, PartialEq)]
pub struct AnimatorState {
    pub name: String,
    pub clip: String,
    pub speed: f32,
}

impl AnimatorState {
    pub fn new(name: impl Into<String>, clip: impl Into<String>) -> Self {
        Self { name: name.into(), clip: clip.into(), speed: 1.0 }
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
}
