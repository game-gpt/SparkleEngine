# spark-types

Shared leaf types across layers: `Vec2` / `Rect` / `Color`, `SparkError` (diagnostics alias), and lightweight 2D lighting grid types.

**Not** the engine core. Geometry intersection / 3D / transforms live in `spark-geometry`.

```bash
cargo test -p spark-types
```
