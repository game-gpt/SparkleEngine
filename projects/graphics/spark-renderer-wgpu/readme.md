# spark-renderer-wgpu

wgpu + winit 后端：泵窗口事件，提交 `DrawList` / `DrawList3d`。

```rust
use spark_renderer::{GameHost, WindowConfig};
use spark_renderer_wgpu::run_window_2d;

run_window_2d(WindowConfig::default(), my_host)?;
```

也提供 `run_window`（空宿主清屏）与 `run_window_3d`。产品路径上更常见 `spark_engine::run_game`：内部做固定步帧循环后再调本
crate。

再导出 `spark-renderer` 抽象类型与 `GlyphCache`。纹理上传走 `TextureUpload`（`create_texture_from_upload`），RGBA8
只是其中一种布局。另有
`mip_level_count`、`uniform_stride`、`binding_size` 等工具函数。错误为 `SparkError`（适配器 / surface / 设备失败等）。

```bash
cargo test -p spark-renderer-wgpu
```
