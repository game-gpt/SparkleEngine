# spark-debugger

调试叠加绘制与帧统计。

```rust
use spark_core::{Color, Rect, Vec2};
use spark_debugger::DebugDraw;

let mut d = DebugDraw::new();
d.rect_filled(Rect::new(0.0, 0.0, 10.0, 10.0), Color::rgb(1.0, 0.0, 0.0));
assert_eq!(d.prims().len(), 1);
d.set_enabled(false);
d.clear();
d.line(Vec2::ZERO, Vec2::new(1.0, 1.0), Color::rgb(1.0, 1.0, 1.0), 1.0);
assert!(d.prims().is_empty());
```

还有 `FrameStats::from_dt`、`DebugSession`、`Inspector` / `NopInspector`。依赖 `spark-renderer` 的颜色矩形等类型，本身不做
GPU 提交。

```bash
cargo test -p spark-debugger
```
