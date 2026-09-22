//! PNG 解码/编码烟测（pure Rust `png`）。

use spark_png::{DecodeOptions, decode_memory, decode_path, encode_path, encode_rgba8};
use spark_texture::{MipmapPolicy, TextureFormat};

#[test]
fn decode_memory_rgba8_srgb() {
    let png = encode_rgba8(1, 1, &[255, 0, 0, 255]).unwrap();
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
    encode_path(&path, 2, 1, &[0, 255, 0, 128, 0, 0, 255, 255]).unwrap();
    let upload = decode_path(&path, DecodeOptions::linear()).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(upload.desc.format, TextureFormat::Rgba8Unorm);
    assert_eq!(upload.desc.width, 2);
    assert_eq!(upload.data.bytes.len(), 8);
}
