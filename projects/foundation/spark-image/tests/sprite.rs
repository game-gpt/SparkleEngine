//! 自 `src/sprite.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_image::*;

use spark_core::Color;

#[test]
fn sheet_index() {
    let img = PixelImage::solid(64, 32, Color::rgb(0.0, 1.0, 0.0)).unwrap();
    let sheet = SpriteSheet::from_image(&img, 4, 2).unwrap();
    let s = sheet.sprite_index(5).unwrap();
    assert_eq!(s.region.x, 16.0);
    assert_eq!(s.region.y, 16.0);
    assert_eq!(s.region.w, 16.0);
    assert_eq!(s.region.h, 16.0);
}
