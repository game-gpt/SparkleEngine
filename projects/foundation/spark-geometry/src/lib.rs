//! Spark 几何库（无游戏语义）。
//!
//! 2D：[`Vec2`] / [`Rect`] 在 `spark-core`；本 crate 提供运算扩展与相交。
//! 3D：[`Vec3`] / [`Mat4`] / [`Quat`] / [`Trs`] / [`Aabb3`] / [`Ray3`] 供渲染与体素局部坐标共用。

mod aabb3;
mod circle;
mod collide;
mod line;
mod mat4;
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
pub use collide::{
    aabb_aabb, circle_aabb, circle_circle, point_in_aabb, point_in_circle, ray_circle,
    segment_segment, ClosestPoint,
};
pub use line::{LineSegment, Ray};
pub use mat4::Mat4;
pub use polygon::Polygon;
pub use quat::Quat;
pub use ray3::{ray_aabb, ray_voxel, ray_voxel_dda, Ray3, VoxelHit};
pub use spark_core::{Rect, Vec2};
pub use sweep3::{aabb_sweep, aabb_sweep_resolve, SweepHit};
pub use transform::Transform2;
pub use trs::Trs;
pub use vec3::Vec3;
pub use vec_ext::Vec2Ext;

#[cfg(test)]
mod tests_3d {
    use super::*;

    #[test]
    fn ray_hits_unit_cube() {
        let ray = Ray3::new(Vec3::new(-2.0, 0.5, 0.5), Vec3::X);
        let t = ray_aabb(ray, Aabb3::from_cell(0, 0, 0)).unwrap();
        assert!((t - 2.0).abs() < 1e-4);
    }

    #[test]
    fn voxel_dda_finds_solid() {
        let hit = ray_voxel_dda(
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::X,
            10.0,
            64,
            |x, _y, _z| x == 3,
        )
        .unwrap();
        assert_eq!(hit.cell, [3, 0, 0]);
        assert_eq!(hit.prev, [2, 0, 0]);
    }

    #[test]
    fn aabb_transform_translation() {
        let a = Aabb3::from_min_max(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        let b = a.transformed(Mat4::translation(Vec3::new(10.0, 0.0, 0.0)));
        assert!((b.min.x - 10.0).abs() < 1e-5);
        assert!((b.max.x - 11.0).abs() < 1e-5);
    }

    #[test]
    fn sweep_hits_wall() {
        let moving = Aabb3::from_center_extents(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.4, 0.4, 0.4));
        let wall = Aabb3::from_min_max(Vec3::new(2.0, 0.0, -1.0), Vec3::new(3.0, 2.0, 1.0));
        let hit = aabb_sweep(moving, Vec3::new(5.0, 0.0, 0.0), wall).unwrap();
        assert!(hit.toi > 0.0 && hit.toi < 1.0);
    }
}
