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

#[test]
fn device_caps_rejects_bc_when_unsupported() {
    let caps = DeviceCaps::conservative();
    assert!(caps.supports_format(TextureFormat::Rgba8UnormSrgb));
    assert!(!caps.supports_format(TextureFormat::Bc7RgbaUnorm));

    let mut upload = TextureUpload::rgba8_srgb(4, 4, vec![0u8; 64]).unwrap().with_mipmap(MipmapPolicy::None);
    upload.desc.format = TextureFormat::Bc7RgbaUnorm;
    upload.desc.color_space = ColorSpace::Linear;
    // 压缩数据长度：4×4 BC7 = 1 block × 16 bytes
    upload.data = TextureData {
        layout: TextureLayout::tightly_packed_2d(TextureFormat::Bc7RgbaUnorm, 4, 4),
        bytes: std::sync::Arc::from(vec![0u8; 16]),
    };
    let err = upload.validate_for_device(&caps).unwrap_err();
    assert_eq!(err.code, codes::texture_format_unsupported());
}

#[test]
fn device_caps_rejects_oversized() {
    let caps = DeviceCaps {
        max_texture_dimension: 64,
        ..DeviceCaps::conservative()
    };
    let upload = TextureUpload::rgba8_srgb(128, 1, vec![0u8; 128 * 4]).unwrap();
    let err = upload.validate_for_device(&caps).unwrap_err();
    assert_eq!(err.code, codes::texture_size_invalid());
}

#[test]
fn sheet_index() {
    let sheet = SpriteSheet::from_size(64, 32, 4, 2).unwrap();
    let s = sheet.sprite_index(5).unwrap();
    assert_eq!(s.region.x, 16.0);
    assert_eq!(s.region.y, 16.0);
    assert_eq!(s.region.w, 16.0);
    assert_eq!(s.region.h, 16.0);
}

#[test]
fn nine_stretch_corners() {
    use spark_core::Rect;
    let nine = NineSlice::new(Rect::new(0.0, 0.0, 32.0, 32.0), Margin::uniform(8.0));
    nine.validate(32, 32).unwrap();
    let quads = nine.layout(Rect::new(0.0, 0.0, 100.0, 60.0)).unwrap();
    assert_eq!(quads.len(), 9);
    assert!((quads[0].src.w - 8.0).abs() < 1e-5);
    assert!((quads[0].dst.w - 8.0).abs() < 1e-5);
    let center = quads.iter().find(|q| (q.dst.x - 8.0).abs() < 1e-5 && (q.dst.y - 8.0).abs() < 1e-5).unwrap();
    assert!((center.dst.w - 84.0).abs() < 1e-5);
    assert!((center.dst.h - 44.0).abs() < 1e-5);
}

#[test]
fn nine_tile_center() {
    use spark_core::Rect;
    let nine = NineSlice::new(Rect::new(0.0, 0.0, 30.0, 30.0), Margin::uniform(10.0)).with_mode(NineSliceMode::Tile);
    let quads = nine.layout(Rect::new(0.0, 0.0, 50.0, 50.0)).unwrap();
    assert!(quads.len() > 9);
}
