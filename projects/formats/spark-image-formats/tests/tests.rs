//! 解码烟测：solid PNG 往返为 TextureUpload。

use spark_image_formats::{DecodeOptions, decode_memory, decode_path};
use spark_texture::{MipmapPolicy, TextureFormat};

#[test]
fn decode_memory_rgba8_srgb() {
    // 1×1 红不透明 PNG
    let png = {
        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
        let mut buf = Vec::new();
        let enc = image::codecs::png::PngEncoder::new(&mut buf);
        use image::ImageEncoder;
        enc.write_image(img.as_raw(), 1, 1, image::ExtendedColorType::Rgba8).unwrap();
        buf
    };
    let upload = decode_memory(&png, DecodeOptions::srgb()).unwrap();
    assert_eq!(upload.desc.width, 1);
    assert_eq!(upload.desc.height, 1);
    assert_eq!(upload.desc.format, TextureFormat::Rgba8UnormSrgb);
    assert_eq!(upload.mipmap, MipmapPolicy::GenerateCpu);
    assert_eq!(upload.data.bytes.as_ref(), &[255, 0, 0, 255]);
}

#[test]
fn decode_path_roundtrip() {
    let path = std::env::temp_dir().join(format!("spark-image-formats-{}.png", std::process::id()));
    let img = image::RgbaImage::from_pixel(2, 1, image::Rgba([0, 255, 0, 128]));
    img.save(&path).unwrap();
    let upload = decode_path(&path, DecodeOptions::linear()).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(upload.desc.format, TextureFormat::Rgba8Unorm);
    assert_eq!(upload.desc.width, 2);
    assert_eq!(upload.data.bytes.len(), 8);
}
