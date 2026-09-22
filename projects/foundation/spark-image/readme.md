# spark-image

过渡期：CPU 像素解码 + 精灵 / 九宫格**几何**（几何只吃宽高，不绑 `PixelImage`）。不碰 GPU。
权威纹理上传见 `spark-texture` / `spark-renderer-wgpu`。

```rust
use spark_core::{Color, Rect};
use spark_image::{Margin, NineSlice, PixelImage};

let img = PixelImage::solid(32, 32, Color::rgb(1.0, 1.0, 1.0)).unwrap();
let nine = NineSlice::new(img.bounds(), Margin::uniform(8.0));
nine.validate(img.width(), img.height()).unwrap();
let quads = nine.layout(Rect::new(0.0, 0.0, 100.0, 60.0)).unwrap();
assert_eq!(quads.len(), 9);
```

`Sprite::uv(w, h)` / `SpriteSheet::from_size` 为推荐入口；`*_image` / `from_image` 为过渡包装。

```bash
cargo test -p spark-image
```
