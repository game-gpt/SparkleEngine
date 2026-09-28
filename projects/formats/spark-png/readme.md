# spark-png

PNG → `TextureUpload` (and RGBA8↔PNG). Pure Rust [`png`](https://crates.io/crates/png) (image-rs/image-png); no umbrella `image`, no `*-sys`.

```rust
use spark_png::{DecodeOptions, decode_memory};

let upload = decode_memory(png_bytes, DecodeOptions::srgb()) ?;
```

```bash
cargo test -p spark-png
```
