# spark-image

过渡期：CPU `PixelImage` + 精灵 / 九宫格几何。解码经 `spark-png` / `spark-jpeg` / `spark-webp`（pure Rust），**不**依赖 umbrella `image`。权威上传见 `spark-texture`。

```rust
use spark_core::{Color, Rect};
use spark_image::{Margin, NineSlice, PixelImage};

let img = PixelImage::solid(32, 32, Color::rgb(1.0, 1.0, 1.0)).unwrap();
let nine = NineSlice::new(img.bounds(), Margin::uniform(8.0));
nine.validate(img.width(), img.height()).unwrap();
let quads = nine.layout(Rect::new(0.0, 0.0, 100.0, 60.0)).unwrap();
assert_eq!(quads.len(), 9);
```

```bash
cargo test -p spark-image
```
