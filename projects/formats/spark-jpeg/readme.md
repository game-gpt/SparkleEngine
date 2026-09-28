# spark-jpeg

JPEG → `TextureUpload`。pure Rust [`jpeg-decoder`](https://crates.io/crates/jpeg-decoder)，不依赖 umbrella `image`，不依赖
libjpeg/`*-sys`。

```rust
use spark_jpeg::{DecodeOptions, decode_memory};

let upload = decode_memory(jpeg_bytes, DecodeOptions::srgb()) ?;
```

```bash
cargo test -p spark-jpeg
```
