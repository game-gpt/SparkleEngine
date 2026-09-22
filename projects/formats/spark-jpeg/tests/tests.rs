//! JPEG 解码烟测（pure Rust `jpeg-decoder`）。

use spark_jpeg::{DecodeOptions, decode_memory};
use spark_texture::TextureFormat;

#[test]
fn decode_memory_rgba8_srgb() {
    let jpeg = include_bytes!("fixtures/red1x1.jpg");
    let upload = decode_memory(jpeg, DecodeOptions::srgb()).unwrap();
    assert_eq!(upload.desc.width, 1);
    assert_eq!(upload.desc.height, 1);
    assert_eq!(upload.desc.format, TextureFormat::Rgba8UnormSrgb);
    assert_eq!(upload.data.bytes.len(), 4);
    // JPEG 有损：只断言偏红且 alpha 不透明。
    assert!(upload.data.bytes[0] > 200);
    assert!(upload.data.bytes[1] < 40);
    assert!(upload.data.bytes[2] < 40);
    assert_eq!(upload.data.bytes[3], 255);
}

#[test]
fn decode_rejects_garbage() {
    let err = decode_memory(b"not-a-jpeg", DecodeOptions::srgb()).unwrap_err();
    assert_eq!(err.to_string(), "spark.image.decode");
}
