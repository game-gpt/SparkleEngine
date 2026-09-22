# spark-webp

WebP → `TextureUpload`。pure Rust [`image-webp`](https://crates.io/crates/image-webp)，不依赖 umbrella `image`，不依赖 libwebp/`*-sys`。

```rust
use spark_webp::{DecodeOptions, decode_memory};

let upload = decode_memory(webp_bytes, DecodeOptions::srgb())?;
```

```bash
cargo test -p spark-webp
```
