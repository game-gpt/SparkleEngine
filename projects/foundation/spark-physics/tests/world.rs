//! 自 `src/world.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_physics::*;

#[test]
fn gravity_moves_dynamic_and_detects_overlap() {
    let mut world = PhysicsWorld::new(PhysicsConfig { gravity: Vec2::new(0.0, 100.0), cell_size: 32.0 });
    let ball = world.spawn(RigidBody2::circle(BodyKind::Dynamic, Vec2::new(0.0, 0.0), 8.0));
    world.step(1.0 / 60.0);
    assert!(world.get(ball).unwrap().position.y > 0.0);

    let a = world.spawn(RigidBody2::aabb(BodyKind::Static, Vec2::new(0.0, 0.0), Vec2::new(20.0, 20.0)));
    let b = world.spawn(RigidBody2::circle(BodyKind::Kinematic, Vec2::new(0.0, 0.0), 5.0));
    world.step(0.0);
    assert!(world.contacts().iter().any(|c| (c.a == a && c.b == b) || (c.a == b && c.b == a)), "expected overlapping pair");
}
