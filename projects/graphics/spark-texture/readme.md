# spark-texture

GPU-agnostic but GPU-friendly texture resource model: descriptors, data layouts, upload packets, samplers, atlas regions, and nine-slice / sprite geometry. **Does not** decode PNG/JPEG/WebP; **does not** depend on `image` / `wgpu`.

```rust
use spark_texture::TextureUpload;

let upload = TextureUpload::rgba8_srgb(2, 2, vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255])
    .unwrap();
assert_eq!(upload.desc.width, 2);
```

Authoritative boundary: `TextureUpload` → backend `GpuTextureHandle`. Source file formats are produced by `projects/formats/` plugins into this crate's types.

```bash
cargo test -p spark-texture
```
