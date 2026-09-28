# spark-font

字体装载与 CPU 字形图集。

```rust
use spark_font::GlyphCache;

let mut cache = GlyphCache::load_system()?;
// 或 GlyphCache::from_path / from_bytes
let info = cache.glyph('A', 16);
let w = cache.measure("Hello", 16);
```

`GlyphInfo` 含 UV、宽高、bearing、advance。图集脏标记供 `spark-renderer-wgpu` 上传。找不到系统字体时返回 `SparkError`，不静默空白。

```bash
cargo test -p spark-font
```
