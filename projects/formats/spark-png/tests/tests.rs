//! PNG 解码烟测。

use spark_png::{DecodeOptions, decode_memory, decode_path};
use spark_texture::{MipmapPolicy, TextureFormat};

fn encode_rgba_png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    let enc = image::codecs::png::PngEncoder::new(&mut buf);
    use image::ImageEncoder;
    enc.write_image(rgba, w, h, image::ExtendedColorType::Rgba8).unwrap();
    buf
}

#[test]
fn decode_memory_rgba8_srgb() {
    let png = encode_rgba_png(1, 1, &[255, 0, 0, 255]);
    let upload = decode_memory(&png, DecodeOptions::srgb()).unwrap();
    assert_eq!(upload.desc.width, 1);
    assert_eq!(upload.desc.height, 1);
    assert_eq!(upload.desc.format, TextureFormat::Rgba8UnormSrgb);
    assert_eq!(upload.mipmap, MipmapPolicy::GenerateCpu);
    assert_eq!(upload.data.bytes.as_ref(), &[255, 0, 0, 255]);
}

#[test]
fn decode_path_linear() {
    let path = std::env::temp_dir().join(format!("spark-png-{}.png", std::process::id()));
    std::fs::write(&path, encode_rgba_png(2, 1, &[0, 255, 0, 128, 0, 0, 255, 255])).unwrap();
    let upload = decode_path(&path, DecodeOptions::linear()).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(upload.desc.format, TextureFormat::Rgba8Unorm);
    assert_eq!(upload.desc.width, 2);
    assert_eq!(upload.data.bytes.len(), 8);
}
