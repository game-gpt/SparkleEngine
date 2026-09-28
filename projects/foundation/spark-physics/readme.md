# spark-physics

2D 刚体物理：`PhysicsWorld`、重力、宽窄相接触。

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

还有 `RigidBody2::aabb`、`BodyKind::{Static, Kinematic, Dynamic}`、`UniformGrid` 等。错误类型 `PhysicsError`。并再导出部分
3D 扫掠原语。

```bash
cargo test -p spark-physics
```
