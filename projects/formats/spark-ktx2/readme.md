# spark-ktx2

KTX2 container parse → `TextureUpload`. Pure Rust [`ktx2`](https://crates.io/crates/ktx2); no umbrella `image`, no `*-sys`.

First slice: no supercompression, `vkFormat` mappable to a subset of `spark-texture::TextureFormat`. BasisLZ / Zstd transcoding is deferred.

```rust
use spark_ktx2::decode_memory;

let upload = decode_memory(ktx2_bytes)?;
```

```bash
cargo test -p spark-ktx2
```
