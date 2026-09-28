# spark-renderer

Backend-agnostic rendering contract: `DrawList`, `FrameCtx`, `Camera2d`, texture uploads, and more. No window, no GPU.

```rust
use spark_types::{Color, Rect};
use spark_renderer::{DrawList, FrameCtx};

// Game logic and frame scheduling live in spark_engine::SparkRuntime;
// this crate only provides draw-command types.
fn fill_demo(draw: &mut DrawList) {
    draw.begin_world();
    draw.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Color::rgb(0.1, 0.1, 0.12));
}
```

3D uses `DrawList3d`. The authoritative upload type is `TextureUpload` (from `spark-texture`).

Window pump contracts are `WindowPump2d` / `WindowPump3d`. Games use `spark_engine::run_runtime`; editor shells can implement `WindowPump2d` and call `run_window_2d`.

```bash
cargo test -p spark-renderer
```
