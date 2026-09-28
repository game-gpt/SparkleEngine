# spark-ecs

Archetype ECS：`World` 管实体与组件，`Resources` 管单例，`Schedule` 按名称跑系统。

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

任意 `Send + Sync + 'static` 类型都是 `Component`。还有 `spawn`、`spawn_empty`、`insert`、`remove`、`despawn`。界面树在
`spark-widget`。

```bash
cargo test -p spark-ecs
```
