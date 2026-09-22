# spark-gltf

glTF 2.0 导入为 CPU 侧资源：网格、可选骨架与动画剪辑。不上 GPU。位于 `projects/formats/`。

```rust
use spark_gltf::import_path;

let asset = import_path("model.gltf")?;
// asset.meshes / skeleton / clips
```

也可用 `import_slice(&[u8])`。错误：`GltfError`。再导出 `Skeleton`、`SkinnedAnimationClip`、`SkinnedVertex`。约定：右手、Y-up、米。绘制走
`spark-renderer` 蒙皮命令。

```bash
cargo test -p spark-gltf
```
