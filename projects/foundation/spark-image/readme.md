# spark-image

CPU 侧像素图、精灵与九宫格布局。不碰 GPU。

```rust
use spark_core::{Color, Rect};
use spark_image::{Margin, NineSlice, PixelImage};

let img = PixelImage::solid(32, 32, Color::rgb(1.0, 1.0, 1.0)).unwrap();
let nine = NineSlice::new(img.bounds(), Margin::uniform(8.0));
nine.validate(&img).unwrap();
let quads = nine.layout(Rect::new(0.0, 0.0, 100.0, 60.0)).unwrap();
assert_eq!(quads.len(), 9);
```

也可用 `PixelImage::load` / `from_rgba8` / `load_from_memory`，以及 `Sprite` / `SpriteSheet`。错误类型为 `SparkError`。

```bash
cargo test -p spark-image
```
