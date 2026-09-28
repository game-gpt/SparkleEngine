# spark-dds

DDS 容器解析 → `TextureUpload`。自研 DXGI / BCn 子集，不依赖 umbrella `image`，不依赖 `*-sys`。

首切：`DXT1` / `DXT5`，以及 DX10 的 `BC1` / `BC3` / `BC5` / `BC7` 与 `R8G8B8A8`（UNORM / SRGB）。

```rust
use spark_dds::decode_memory;

let upload = decode_memory(dds_bytes)?;
```

```bash
cargo test -p spark-dds
```
