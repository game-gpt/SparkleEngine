# spark-engine-rts

即时战略骨架：单位名册、框选、指令队列、迷雾。

```rust
use spark_core::Vec2;
use spark_engine_rts::{Command, PlayerId, RtsEngine, UnitPose};

let mut rts = RtsEngine::new(".", 32, 32, 1.0);
let p = PlayerId(1);
let a = rts.roster.spawn(p, UnitPose::at(2.0, 2.0), 4.0);
rts.box_select(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0), Some(p));
rts.commands.issue(a, Command::MoveTo { target: Vec2::new(8.0, 2.0), speed: 4.0 });
rts.tick(1.0, p);
```

科技树与兵种表由游戏提供。

```bash
cargo test -p spark-engine-rts
```
