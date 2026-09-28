# spark-font

Font loading and CPU glyph atlas.

```rust
use spark_font::GlyphCache;

let mut cache = GlyphCache::load_system()?;
// or GlyphCache::from_path / from_bytes
let info = cache.glyph('A', 16);
let w = cache.measure("Hello", 16);
```

`GlyphInfo` includes UV, size, bearing, advance. Atlas dirty flag is for `spark-renderer-wgpu` upload. Returns `SparkError` when system fonts are missing; does not silently render blank.

```bash
cargo test -p spark-font
```
