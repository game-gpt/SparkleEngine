# spark-engine-rts

Real-time strategy skeleton: unit roster, box select, command queue, fog of war.

```rust
use spark_types::Vec2;
use spark_engine_rts::{Command, PlayerId, RtsEngine, UnitPose};

let mut rts = RtsEngine::new(".", 32, 32, 1.0);
let p = PlayerId(1);
let a = rts.roster.spawn(p, UnitPose::at(2.0, 2.0), 4.0);
rts.box_select(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0), Some(p));
rts.commands.issue(a, Command::MoveTo { target: Vec2::new(8.0, 2.0), speed: 4.0 });
rts.tick(1.0, p);
```

Tech trees and unit tables are provided by the game.

```bash
cargo test -p spark-engine-rts
```
