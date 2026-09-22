# spark-png

PNG → `TextureUpload`。不暴露 `PixelImage`，不碰 GPU。

```rust
use spark_png::{DecodeOptions, decode_memory};

let upload = decode_memory(png_bytes, DecodeOptions::srgb())?;
```

```bash
cargo test -p spark-png
```
