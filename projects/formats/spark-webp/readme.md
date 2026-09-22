# spark-webp

WebP → `TextureUpload`。不暴露 `PixelImage`，不碰 GPU。

```rust
use spark_webp::{DecodeOptions, decode_memory};

let upload = decode_memory(webp_bytes, DecodeOptions::srgb())?;
```

```bash
cargo test -p spark-webp
```
