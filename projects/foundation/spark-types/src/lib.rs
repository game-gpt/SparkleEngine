//! Spark 公共基础类型。不含游戏玩法概念。
//!
//! 错误使用 [`SparkError`]（即 `spark-diagnostics::Error`）：只保存稳定码与类型化参数，
//! **不**把自然语言句子作为错误权威内容。
//!
//! 本 crate 原名不宜叫 `core`：它只承载跨层共享的类型与错误别名，不是「引擎内核」。

#![warn(missing_docs)]
pub use spark_diagnostics::{Diagnostic, ErrorArg, ErrorArgs, ErrorCode, ErrorContext, MessageKey, Severity, codes};

mod light2d;
mod vec2;

pub use light2d::{LightFalloff, LightGrid2d, LightRgb};
pub use vec2::Vec2;

/// 引擎级结构化错误（稳定码 + 参数；`Display` 仅输出错误码）。
pub type SparkError = spark_diagnostics::Error;

/// 轴对齐矩形（屏幕或世界坐标由调用方约定）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// 左。
    pub x: f32,
    /// 上。
    pub y: f32,
    /// 宽。
    pub w: f32,
    /// 高。
    pub h: f32,
}

impl Rect {
    /// 构造。
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// 点是否在矩形内（半开区间，右下不含）。
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.x && p.y >= self.y && p.x < self.x + self.w && p.y < self.y + self.h
    }

    /// 中心点。
    pub fn center(self) -> Vec2 {
        Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    /// 与另一矩形求交。无交集时宽或高为 0。
    pub fn intersect(self, other: Self) -> Self {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.w).min(other.x + other.w);
        let y1 = (self.y + self.h).min(other.y + other.h);
        Self { x: x0, y: y0, w: (x1 - x0).max(0.0), h: (y1 - y0).max(0.0) }
    }

    /// 宽或高是否非正。
    pub fn is_empty(self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }

    /// 是否与另一矩形相交。
    pub fn intersects(self, other: Self) -> bool {
        !self.intersect(other).is_empty()
    }
}

/// RGBA，分量 0.0–1.0。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// 红。
    pub r: f32,
    /// 绿。
    pub g: f32,
    /// 蓝。
    pub b: f32,
    /// 透明。
    pub a: f32,
}

impl Color {
    /// 含 alpha。
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// 不透明。
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    /// 转数组。
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}
