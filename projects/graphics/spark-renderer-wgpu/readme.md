# spark-renderer-wgpu

wgpu + winit backend: pumps window events and submits `DrawList` / `DrawList3d`.

```rust
use spark_renderer::{GameHost, WindowConfig};
use spark_renderer_wgpu::run_window_2d;

run_window_2d(WindowConfig::default(), my_host)?;
```

Also provides `run_window` (empty host, clear only) and `run_window_3d`. On product paths, `spark_engine::run_game` is more common: it runs a fixed-step frame loop before calling this crate.

Re-exports `spark-renderer` abstract types and `GlyphCache`. Texture uploads go through `TextureUpload` (`create_texture_from_upload`); RGBA8 is one layout among others. Utility helpers include `mip_level_count`, `uniform_stride`, `binding_size`. Errors are `SparkError` (adapter / surface / device failures, etc.).

```bash
cargo test -p spark-renderer-wgpu
```
