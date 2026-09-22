# spark-renderer

后端无关的绘制契约与游戏宿主 trait。不含窗口、不含 GPU。

```rust
use spark_core::{Color, Rect};
use spark_renderer::{DrawList, FrameCtx, GameHost};

struct MyHost;

impl GameHost for MyHost {
    fn update(&mut self, ctx: &FrameCtx<'_>) {
        let _ = ctx.input;
        let _ = ctx.dt;
    }

    fn draw(&mut self, draw: &mut DrawList) {
        draw.begin_world();
        draw.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Color::rgb(0.1, 0.1, 0.12));
    }
}
```

3D 用 `GameHost3d` / `DrawList3d`。纹理权威上传类型为 `TextureUpload`（来自 `spark-texture`）。开窗提交用 `spark_engine::run_game` 或 `spark_renderer_wgpu::run_window_2d`。

```bash
cargo test -p spark-renderer
```
