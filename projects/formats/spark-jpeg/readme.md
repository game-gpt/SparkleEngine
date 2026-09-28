# spark-jpeg

JPEG → `TextureUpload`. Pure Rust [`jpeg-decoder`](https://crates.io/crates/jpeg-decoder); no umbrella `image`, no libjpeg/`*-sys`.

```rust
use spark_jpeg::{DecodeOptions, decode_memory};

let upload = decode_memory(jpeg_bytes, DecodeOptions::srgb()) ?;
```

```bash
cargo test -p spark-jpeg
```
