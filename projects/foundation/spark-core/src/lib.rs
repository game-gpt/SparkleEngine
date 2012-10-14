//! Spark 基础类型。不含游戏玩法概念。
//!
//! 错误使用 [`SparkError`]（即 `spark-diagnostics::Error`）：只保存稳定码与类型化参数，
//! **不**把自然语言句子作为错误权威内容。

pub use spark_diagnostics::{
    Diagnostic, ErrorArg, ErrorArgs, ErrorCode, ErrorContext, MessageKey, Severity, codes,
};

mod light2d;

pub use light2d::{LightFalloff, LightGrid2d, LightRgb};

/// 引擎级结构化错误（稳定码 + 参数；`Display` 仅输出错误码）。
pub type SparkError = spark_diagnostics::Error;

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

/// 轴对齐矩形（屏幕或世界坐标由调用方约定）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.x && p.y >= self.y && p.x < self.x + self.w && p.y < self.y + self.h
    }

    pub fn center(self) -> Vec2 {
        Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    /// 与另一矩形求交。无交集时宽或高为 0。
    pub fn intersect(self, other: Self) -> Self {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.w).min(other.x + other.w);
        let y1 = (self.y + self.h).min(other.y + other.h);
        Self {
            x: x0,
            y: y0,
            w: (x1 - x0).max(0.0),
            h: (y1 - y0).max(0.0),
        }
    }

    pub fn is_empty(self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }

    pub fn intersects(self, other: Self) -> bool {
        !self.intersect(other).is_empty()
    }
}

/// RGBA，分量 0.0–1.0。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}
