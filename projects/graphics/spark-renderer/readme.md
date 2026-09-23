# spark-renderer

后端无关的绘制契约：`DrawList`、`FrameCtx`、`Camera2d`、纹理上传等。不含窗口、不含 GPU。

```rust
use spark_types::{Color, Rect};
use spark_renderer::{DrawList, FrameCtx};

// 游戏逻辑与帧调度在 spark_engine::SparkRuntime；
// 本 crate 只提供绘制命令类型。
fn fill_demo(draw: &mut DrawList) {
    draw.begin_world();
    draw.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Color::rgb(0.1, 0.1, 0.12));
}
```

3D 用 `DrawList3d`。纹理权威上传类型为 `TextureUpload`（来自 `spark-texture`）。

`GameHost` / `GameHost3d` 为遗留窗口泵 trait（由 `RuntimeHost2d` 实现）；**禁止**游戏类型直接实现。开窗提交用 `spark_engine::run_runtime`。

```bash
cargo test -p spark-renderer
```
