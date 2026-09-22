# spark-image-formats

源图片格式（PNG / JPEG / WebP 等）→ `TextureUpload`。不暴露 CPU `PixelImage`，不碰 GPU。

```rust
use spark_image_formats::{DecodeOptions, decode_memory};

let upload = decode_memory(png_bytes, DecodeOptions::srgb())?;
// 交给 DrawList::create_texture_upload / wgpu create_texture_from_upload
```

运行时基础 crate（`spark-texture` / `spark-renderer`）**不得**依赖本包或 `image`。

```bash
cargo test -p spark-image-formats
```
