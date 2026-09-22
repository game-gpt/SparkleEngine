//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_core::Color;
use spark_image::*;

#[test]
fn solid_and_uv() {
    let img = PixelImage::solid(64, 32, Color::rgb(1.0, 0.0, 0.0)).unwrap();
    assert_eq!(img.width(), 64);
    assert_eq!(img.pixel(0, 0).unwrap(), [255, 0, 0, 255]);
    let uv = img.uv_rect(Rect::new(16.0, 8.0, 16.0, 8.0)).unwrap();
    assert!((uv.x - 0.25).abs() < 1e-5);
    assert!((uv.w - 0.25).abs() < 1e-5);
}

#[test]
fn errors_are_stable_codes() {
    let err = PixelImage::from_rgba8(1, 1, vec![0, 0, 0]).unwrap_err();
    assert_eq!(err.to_string(), "spark.image.rgba_length_mismatch");
}

#[test]
fn png_roundtrip() {
    let src = PixelImage::solid(2, 1, Color::rgba(0.0, 1.0, 0.0, 0.5)).unwrap();
    let path = std::env::temp_dir().join(format!("spark-image-{}.png", std::process::id()));
    src.save_png(&path).unwrap();
    let loaded = PixelImage::load(&path).unwrap();
    assert_eq!(loaded.width(), 2);
    assert_eq!(loaded.height(), 1);
    assert_eq!(loaded.pixel(0, 0).unwrap(), src.pixel(0, 0).unwrap());
    let _ = std::fs::remove_file(&path);
}
