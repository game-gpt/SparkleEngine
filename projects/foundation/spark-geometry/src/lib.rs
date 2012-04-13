//! Spark 几何库（无游戏语义）。
//!
//! 2D：[`Vec2`] / [`Rect`] 在 `spark-core`；本 crate 提供运算扩展与相交。
//! 3D：[`Vec3`] / [`Mat4`] 供渲染与体素局部坐标共用。

mod circle;
mod collide;
mod line;
mod mat4;
mod polygon;
mod transform;
mod vec3;
mod vec_ext;

pub use circle::Circle;
pub use collide::{
    aabb_aabb, circle_aabb, circle_circle, point_in_aabb, point_in_circle, ray_circle,
    segment_segment, ClosestPoint,
};
pub use line::{LineSegment, Ray};
pub use mat4::Mat4;
pub use polygon::Polygon;
pub use spark_core::{Rect, Vec2};
pub use transform::Transform2;
pub use vec3::Vec3;
pub use vec_ext::Vec2Ext;
