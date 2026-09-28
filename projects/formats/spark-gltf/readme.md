# spark-gltf

glTF 2.0 import to CPU-side resources: meshes, optional skeleton and animation clips. No GPU upload. Lives under `projects/formats/`.

```rust
use spark_gltf::import_path;

let asset = import_path("model.gltf")?;
// asset.meshes / skeleton / clips
```

Also `import_slice(&[u8])`. Errors: `GltfError`. Re-exports `Skeleton`, `SkinnedAnimationClip`, `SkinnedVertex`. Convention: right-handed, Y-up, meters. Drawing goes through `spark-renderer` skinned commands.

```bash
cargo test -p spark-gltf
```
