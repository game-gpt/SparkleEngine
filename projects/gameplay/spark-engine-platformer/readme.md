# spark-engine-platformer

Platformer skeleton: solid platforms, character body, `tick` advancement.

```rust
use spark_types::{Rect, Vec2};
use spark_engine_platformer::{ControllerInput, PlatformerEngine, SolidKind, SolidRect};

let mut eng = PlatformerEngine::new(".");
eng.world.push(SolidRect {
    rect: Rect::new(0.0, 0.0, 20.0, 1.0),
    kind: SolidKind::Solid,
});
eng.player.pos = Vec2::new(2.0, 3.0);
eng.tick(1.0 / 60.0, ControllerInput::default());
```

Types also include `PlatformerConfig`, `ActorBody`, camera dead zones, etc. Level tile tables and progression tuning are provided by the game.

```bash
cargo test -p spark-engine-platformer
```
