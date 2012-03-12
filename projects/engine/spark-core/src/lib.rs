//! Spark 基础类型。不含游戏玩法概念。

#![forbid(unsafe_code)]

use thiserror::Error;

/// 二维向量（逻辑 / 呈现共用基础表示）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// 引擎级错误根类型。
#[derive(Debug, Error)]
pub enum SparkError {
    #[error("尚未实现：{0}")]
    NotImplemented(&'static str),
}
