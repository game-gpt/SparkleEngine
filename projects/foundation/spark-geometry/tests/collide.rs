//! 自 `src/collide.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_geometry::*;

#[test]
fn circles_and_ray() {
    let a = Circle::new(Vec2::new(0.0, 0.0), 1.0);
    let b = Circle::new(Vec2::new(1.5, 0.0), 1.0);
    assert!(circle_circle(a, b));
    let ray = Ray::new(Vec2::new(-5.0, 0.0), Vec2::new(1.0, 0.0));
    let t = ray_circle(ray, a).unwrap();
    assert!((t - 4.0).abs() < 1e-4);
}

#[test]
fn aabb() {
    let a = Rect::new(0.0, 0.0, 2.0, 2.0);
    let b = Rect::new(1.0, 1.0, 2.0, 2.0);
    assert!(aabb_aabb(a, b));
    assert!(circle_aabb(Circle::new(Vec2::new(3.0, 1.0), 1.1), a));
}
