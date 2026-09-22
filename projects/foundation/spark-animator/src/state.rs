//! 状态机中的一个节点。它指向剪辑名，不持有剪辑数据。

/// 状态机节点：名字、引用的剪辑名、播放倍率。
#[derive(Debug, Clone, PartialEq)]
pub struct AnimatorState {
    /// 状态名，过渡的 `from` / `to` 与此匹配。
    pub name: String,
    /// [`crate::ClipLibrary`] 中的剪辑名。
    pub clip: String,
    /// 进入该态时写入播放器的速度倍率。
    pub speed: f32,
}

impl AnimatorState {
    /// 新状态，默认速度 `1.0`。
    pub fn new(name: impl Into<String>, clip: impl Into<String>) -> Self {
        Self { name: name.into(), clip: clip.into(), speed: 1.0 }
    }

    /// 覆盖播放倍率后返回自身（建造者风格）。
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
}
