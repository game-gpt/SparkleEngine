//! 平台跳跃壳沿用渲染层的 [`Camera2d`]。死区参数留在本模块，避免再定义第二套相机。

pub use spark_renderer::Camera2d;

use spark_core::Vec2;

/// 与旧壳相同的默认死区（世界单位）和插值速度。
pub const DEADZONE: Vec2 = Vec2::new(2.0, 1.5);
pub const FOLLOW_LERP: f32 = 8.0;
