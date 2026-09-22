# spark-texture

GPU 无关但 GPU 友好的纹理资源模型：描述、数据布局、上传包、采样器、图集区域与九宫格 / 精灵几何。 **不**解码 PNG/JPEG/WebP，
**不**依赖 `image` / `wgpu`。

```rust
use spark_texture::TextureUpload;

let upload = TextureUpload::rgba8_srgb(2, 2, vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255])
    .unwrap();
assert_eq!(upload.desc.width, 2);
```

权威边界：`TextureUpload` → 后端 `GpuTextureHandle`。源文件格式由 `projects/formats/` 插件产出本 crate 类型。

```bash
cargo test -p spark-texture
```
