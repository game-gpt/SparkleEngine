//! `spark-texture` 集成测试。

use spark_core::codes;
use spark_texture::*;

#[test]
fn rgba8_srgb_upload_validates() {
    let rgba = vec![255u8, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255];
    let upload = TextureUpload::rgba8_srgb(2, 2, rgba).unwrap();
    upload.validate().unwrap();
    assert_eq!(upload.desc.width, 2);
    assert_eq!(upload.desc.format, TextureFormat::Rgba8UnormSrgb);
    assert_eq!(upload.mipmap, MipmapPolicy::GenerateCpu);
    assert_eq!(upload.data.bytes.len(), 16);
}

#[test]
fn rgba8_length_mismatch() {
    let err = TextureUpload::rgba8_srgb(1, 1, vec![0u8, 0, 0]).unwrap_err();
    assert_eq!(err.code, codes::texture_data_length_mismatch());
}

#[test]
fn compressed_rejects_generate_cpu() {
    let mut upload = TextureUpload::rgba8_srgb(4, 4, vec![0u8; 64]).unwrap();
    upload.desc.format = TextureFormat::Bc7RgbaUnorm;
    let err = upload.validate().unwrap_err();
    assert_eq!(err.code, codes::texture_upload_invalid());
}

#[test]
fn sprite_region_uv() {
    let info = TextureInfo { width: 64, height: 32, format: TextureFormat::Rgba8UnormSrgb, mip_levels: 1 };
    let region = SpriteRegion {
        pixel_rect: spark_core::Rect::new(16.0, 0.0, 16.0, 16.0),
        uv_rect: None,
        pivot: spark_core::Vec2::ZERO,
        logical_size: spark_core::Vec2::new(16.0, 16.0),
    };
    let uv = region.uv_for(info);
    assert!((uv.x - 0.25).abs() < 1e-5);
    assert!((uv.w - 0.25).abs() < 1e-5);
}

#[test]
fn format_block_meta() {
    assert!(TextureFormat::Bc7RgbaUnorm.is_compressed());
    assert_eq!(TextureFormat::Bc7RgbaUnorm.block_extent(), (4, 4));
    assert_eq!(TextureFormat::Rgba8Unorm.bytes_per_block(), 4);
}
