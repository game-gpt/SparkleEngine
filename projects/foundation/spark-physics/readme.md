# spark-physics

2D rigid-body physics: `PhysicsWorld`, gravity, broad/narrow-phase contacts.

```rust
use spark_physics::{BodyKind, PhysicsConfig, PhysicsWorld, RigidBody2, Vec2};

let mut world = PhysicsWorld::new(PhysicsConfig {
    gravity: Vec2::new(0.0, 100.0),
    cell_size: 32.0,
});
let ball = world.spawn(RigidBody2::circle(BodyKind::Dynamic, Vec2::new(0.0, 0.0), 8.0));
world.step(1.0 / 60.0);
assert!(world.get(ball).unwrap().position.y > 0.0);
```

Also `RigidBody2::aabb`, `BodyKind::{Static, Kinematic, Dynamic}`, `UniformGrid`, etc. Error type `PhysicsError`. Re-exports some 3D sweep primitives.

```bash
cargo test -p spark-physics
```
