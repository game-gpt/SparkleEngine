# spark-ecs

Archetype ECS: `World` manages entities and components, `Resources` holds singletons, `Schedule` runs systems by name.

```rust
use spark_ecs::{Schedule, World};

#[derive(Debug, PartialEq)]
struct Pos(f32);
#[derive(Debug, PartialEq)]
struct Vel(f32);

let mut world = World::new();
let e = world.spawn2(Pos(1.0), Vel(2.0));
world.for_each2_mut::<Pos, Vel>(|_, p, v| {
    p.0 += v.0;
});
assert_eq!(world.get::<Pos>(e).unwrap().0, 3.0);

let mut schedule = Schedule::new();
schedule.add_fn("tick", |_w| {});
schedule.run(&mut world);
```

Any `Send + Sync + 'static` type is a `Component`. Also: `spawn`, `spawn_empty`, `insert`, `remove`, `despawn`. UI trees live in `spark-widget`.

```bash
cargo test -p spark-ecs
```
