# spark-jpeg

JPEG → `TextureUpload`。不暴露 `PixelImage`，不碰 GPU。

```rust
use spark_jpeg::{DecodeOptions, decode_memory};

let upload = decode_memory(jpeg_bytes, DecodeOptions::srgb())?;
```

```bash
cargo test -p spark-jpeg
```
