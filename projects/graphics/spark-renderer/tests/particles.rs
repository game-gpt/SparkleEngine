//! 自 `src/particles.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_core::{Color, Vec2};
use spark_renderer::*;

#[test]
fn tick_moves_and_expires() {
    let mut pool = ParticlePool2d::with_capacity(1);
    pool.spawn(Particle2d::new(Vec2::ZERO, Vec2::new(10.0, 0.0), 0.5, Color::rgb(1.0, 1.0, 1.0), 2.0));
    pool.tick(0.1);
    assert!((pool.slots()[0].pos.x - 1.0).abs() < 1e-4);
    pool.tick(1.0);
    assert!(pool.is_empty());
    pool.spawn(Particle2d::new(Vec2::new(3.0, 0.0), Vec2::ZERO, 1.0, Color::rgb(1.0, 0.0, 0.0), 4.0));
    assert_eq!(pool.len(), 1);
    assert!((pool.slots()[0].pos.x - 3.0).abs() < 1e-4);
}
