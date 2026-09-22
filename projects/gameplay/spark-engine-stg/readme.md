# spark-engine-stg

弹幕骨架：子弹池、发射器、关卡时钟。

```rust
use spark_core::Vec2;
use spark_engine_stg::{EmitPattern, Emitter, StageClock, StgEngine};

let mut stg = StgEngine::new(".", 256);
let em = Emitter {
    pattern: EmitPattern::Fan {
        count: 5,
        spread_rad: std::f32::consts::FRAC_PI_2,
        speed: 100.0,
    },
    bullet_radius: 2.0,
    layer: 0,
};
stg.emit(&em, Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
assert_eq!(stg.bullets.alive_count(), 5);
let hit = stg.tick(0.0, Vec2::new(0.0, 0.0), 4.0, 8.0);

let mut clock = StageClock::default();
clock.advance(0.5);
```

弹种表与符卡由游戏提供。

```bash
cargo test -p spark-engine-stg
```
