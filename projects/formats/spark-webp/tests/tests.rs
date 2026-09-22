//! WebP 解码烟测（pure Rust `image-webp`）。

use spark_texture::TextureFormat;
use spark_webp::{DecodeOptions, decode_memory};

#[test]
fn decode_rejects_non_webp() {
    let err = decode_memory(b"not-a-webp", DecodeOptions::srgb()).unwrap_err();
    assert_eq!(err.to_string(), "spark.image.decode");
}

#[test]
fn decode_fixture_rgba() {
    let bytes = include_bytes!("fixtures/green2x1.webp");
    let upload = decode_memory(bytes, DecodeOptions::srgb()).unwrap();
    assert_eq!(upload.desc.format, TextureFormat::Rgba8UnormSrgb);
    assert_eq!(upload.desc.width, 2);
    assert_eq!(upload.desc.height, 1);
    assert_eq!(upload.data.bytes.len(), 8);
}
