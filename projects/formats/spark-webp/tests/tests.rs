//! WebP 解码烟测（无编码器时跳过）。

use spark_texture::TextureFormat;
use spark_webp::{DecodeOptions, decode_memory};

#[test]
fn decode_rejects_non_webp() {
    let err = decode_memory(b"not-a-webp", DecodeOptions::srgb()).unwrap_err();
    assert_eq!(err.to_string(), "spark.image.decode");
}

#[test]
fn decode_fixture_if_present() {
    // 避免强依赖 WebP 编码器：仅在环境提供样例时验证像素路径。
    let Ok(bytes) = std::fs::read(std::env::var_os("SPARK_WEBP_FIXTURE").unwrap_or_default())
    else {
        return;
    };
    let upload = decode_memory(&bytes, DecodeOptions::srgb()).unwrap();
    assert_eq!(upload.desc.format, TextureFormat::Rgba8UnormSrgb);
    assert!(upload.desc.width > 0);
    assert_eq!(upload.data.bytes.len() as u32, upload.desc.width * upload.desc.height * 4);
}
