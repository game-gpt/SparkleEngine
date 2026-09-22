# spark-png

PNG → `TextureUpload`（及 RGBA8↔PNG）。pure Rust [`png`](https://crates.io/crates/png)（image-rs/image-png），不依赖 umbrella `image`，不依赖 `*-sys`。

```rust
use spark_png::{DecodeOptions, decode_memory};

let upload = decode_memory(png_bytes, DecodeOptions::srgb())?;
```

```bash
cargo test -p spark-png
```
