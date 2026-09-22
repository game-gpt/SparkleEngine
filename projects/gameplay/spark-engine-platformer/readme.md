# spark-engine-platformer

平台跳跃骨架：固体台、角色体、`tick` 推进。

```rust
use spark_core::{Rect, Vec2};
use spark_engine_platformer::{ControllerInput, PlatformerEngine, SolidKind, SolidRect};

let mut eng = PlatformerEngine::new(".");
eng.world.push(SolidRect {
    rect: Rect::new(0.0, 0.0, 20.0, 1.0),
    kind: SolidKind::Solid,
});
eng.player.pos = Vec2::new(2.0, 3.0);
eng.tick(1.0 / 60.0, ControllerInput::default());
```

类型还包括 `PlatformerConfig`、`ActorBody`、相机死区等。关卡块表与成长数值由游戏提供。

```bash
cargo test -p spark-engine-platformer
```
