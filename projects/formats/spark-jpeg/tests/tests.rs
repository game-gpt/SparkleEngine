//! JPEG 解码烟测。

use spark_jpeg::{DecodeOptions, decode_memory};
use spark_texture::TextureFormat;

fn encode_rgb_jpeg(w: u32, h: u32, rgb: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90);
    use image::ImageEncoder;
    enc.write_image(rgb, w, h, image::ExtendedColorType::Rgb8).unwrap();
    buf
}

#[test]
fn decode_memory_rgba8_srgb() {
    let jpeg = encode_rgb_jpeg(1, 1, &[255, 0, 0]);
    let upload = decode_memory(&jpeg, DecodeOptions::srgb()).unwrap();
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
