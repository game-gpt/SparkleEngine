//! Spark 2D 几何库（无游戏语义）。
//!
//! 基础类型 [`Vec2`] / [`Rect`] 仍定义在 `spark-core`；本 crate 提供运算扩展、
//! 圆 / 线段 / 射线 / 变换，以及常用相交与距离查询。供物理、STG、RTS、渲染共用。

mod circle;
mod collide;
mod line;
mod polygon;
mod transform;
mod vec_ext;

pub use circle::Circle;
pub use collide::{
    aabb_aabb, circle_aabb, circle_circle, point_in_aabb, point_in_circle, ray_circle,
    segment_segment, ClosestPoint,
};
pub use line::{LineSegment, Ray};
pub use polygon::Polygon;
pub use spark_core::{Rect, Vec2};
pub use transform::Transform2;
pub use vec_ext::Vec2Ext;
