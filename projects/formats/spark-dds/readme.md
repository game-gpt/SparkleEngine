# spark-dds

DDS container parse → `TextureUpload`. In-house DXGI / BCn subset; no umbrella `image`, no `*-sys`.

First slice: `DXT1` / `DXT5`, plus DX10 `BC1` / `BC3` / `BC5` / `BC7` and `R8G8B8A8` (UNORM / SRGB).

```rust
use spark_dds::decode_memory;

let upload = decode_memory(dds_bytes)?;
```

```bash
cargo test -p spark-dds
```
