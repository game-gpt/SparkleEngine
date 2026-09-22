# spark-ktx2

KTX2 容器解析 → `TextureUpload`。pure Rust [`ktx2`](https://crates.io/crates/ktx2)，不依赖 umbrella `image`，不依赖 `*-sys`。

首切：无超级压缩、`vkFormat` 可映射到 `spark-texture::TextureFormat` 的子集。BasisLZ / Zstd 转码后置。

```rust
use spark_ktx2::decode_memory;

let upload = decode_memory(ktx2_bytes)?;
```

```bash
cargo test -p spark-ktx2
```
