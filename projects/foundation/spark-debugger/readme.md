# spark-debugger

Debug overlay drawing and frame statistics.

```rust
use spark_types::{Color, Rect, Vec2};
use spark_debugger::DebugDraw;

let mut d = DebugDraw::new();
d.rect_filled(Rect::new(0.0, 0.0, 10.0, 10.0), Color::rgb(1.0, 0.0, 0.0));
assert_eq!(d.prims().len(), 1);
d.set_enabled(false);
d.clear();
d.line(Vec2::ZERO, Vec2::new(1.0, 1.0), Color::rgb(1.0, 1.0, 1.0), 1.0);
assert!(d.prims().is_empty());
```

Also `FrameStats::from_dt`, `DebugSession`, `Inspector` / `NopInspector`. Depends on `spark-renderer` colored-rect types; does not submit to GPU itself.

```bash
cargo test -p spark-debugger
```
