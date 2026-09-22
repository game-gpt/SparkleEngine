//! 自 `src/nine.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_core::Color;
use spark_image::*;

#[test]
fn nine_stretch_corners() {
    let img = PixelImage::solid(32, 32, Color::rgb(1.0, 1.0, 1.0)).unwrap();
    let nine = NineSlice::new(img.bounds(), Margin::uniform(8.0));
    nine.validate(&img).unwrap();
    let quads = nine.layout(Rect::new(0.0, 0.0, 100.0, 60.0)).unwrap();
    // 3×3 全在
    assert_eq!(quads.len(), 9);
    // 左上角源 8×8，目标 8×8
    assert!((quads[0].src.w - 8.0).abs() < 1e-5);
    assert!((quads[0].dst.w - 8.0).abs() < 1e-5);
    // 中心目标宽 = 100 - 16
    let center = quads.iter().find(|q| (q.dst.x - 8.0).abs() < 1e-5 && (q.dst.y - 8.0).abs() < 1e-5).unwrap();
    assert!((center.dst.w - 84.0).abs() < 1e-5);
    assert!((center.dst.h - 44.0).abs() < 1e-5);
}

#[test]
fn nine_tile_center() {
    let nine = NineSlice::new(Rect::new(0.0, 0.0, 30.0, 30.0), Margin::uniform(10.0)).with_mode(NineSliceMode::Tile);
    let quads = nine.layout(Rect::new(0.0, 0.0, 50.0, 50.0)).unwrap();
    // 中心 30×30 目标，源中心 10×10 → 至少 9 片中心瓦 + 边角
    assert!(quads.len() > 9);
}
