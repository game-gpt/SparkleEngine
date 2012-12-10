//! 球拍（Rust 权威组件，供类型注册 / Inspector 元数据对接）。

#[derive(Debug, Clone)]
pub struct Paddle {
    pub speed: f32,
    pub side: PaddleSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddleSide {
    Left,
    Right,
}

impl Default for Paddle {
    fn default() -> Self {
        Self {
            speed: 420.0,
            side: PaddleSide::Left,
        }
    }
}
