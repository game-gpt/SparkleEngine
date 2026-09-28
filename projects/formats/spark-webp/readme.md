# spark-webp

WebP → `TextureUpload`. Pure Rust [`image-webp`](https://crates.io/crates/image-webp); no umbrella `image`, no libwebp/`*-sys`.

```rust
use spark_webp::{DecodeOptions, decode_memory};

let upload = decode_memory(webp_bytes, DecodeOptions::srgb()) ?;
```

```bash
cargo test -p spark-webp
```
