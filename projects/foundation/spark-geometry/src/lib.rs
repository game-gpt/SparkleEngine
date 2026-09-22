//! Spark 几何库（无游戏语义）。
//!
//! 2D：[`Vec2`] / [`Rect`] 在 `spark-core`；本 crate 提供运算扩展与相交。
//! 3D：[`Vec3`] / [`Mat4`] / [`Quat`] / [`Trs`] / [`Aabb3`] / [`Ray3`] 供渲染与体素局部坐标共用。

#![warn(missing_docs)]
mod aabb3;
mod circle;
mod collide;
mod line;
mod mat4;
mod periodic;
mod polygon;
mod quat;
mod ray3;
mod sweep3;
mod transform;
mod trs;
mod vec3;
mod vec_ext;

pub use aabb3::Aabb3;
pub use circle::Circle;
pub use collide::{ClosestPoint, aabb_aabb, circle_aabb, circle_circle, point_in_aabb, point_in_circle, ray_circle, segment_segment};
pub use line::{LineSegment, Ray};
pub use mat4::Mat4;
pub use periodic::{PeriodSize, PeriodicAxes, floor_mod, normalize_position, periodic_distance, shortest_delta, shortest_delta_1d};
pub use polygon::Polygon;
pub use quat::Quat;
pub use ray3::{Ray3, VoxelHit, ray_aabb, ray_voxel, ray_voxel_dda};
pub use spark_core::{Rect, Vec2};
pub use sweep3::{SweepHit, aabb_sweep, aabb_sweep_resolve};
pub use transform::Transform2;
pub use trs::Trs;
pub use vec_ext::Vec2Ext;
pub use vec3::Vec3;
