//! KTX2 → TextureUpload 烟测（夹具用手写 Header + DFD 生成）。

use spark_ktx2::decode_memory;
use spark_texture::{MipmapPolicy, TextureFormat};

/// 构造无超级压缩、单 mip、1×1 `R8G8B8A8_SRGB` KTX2。
fn make_rgba8_srgb_1x1(pixel: [u8; 4]) -> Vec<u8> {
    let format = ktx2::Format::R8G8B8A8_SRGB;
    let (basic, type_size) = ktx2::dfd::Basic::from_format(format).expect("DFD for R8G8B8A8_SRGB");
    let block = ktx2::dfd::Block::Basic(basic);
    let block_bytes = block.to_vec();
    let dfd_byte_length = (4 + block_bytes.len()) as u32;

    let header_len = ktx2::Header::LENGTH;
    let level_index_len = ktx2::LevelIndex::LENGTH;
    let dfd_byte_offset = (header_len + level_index_len) as u32;
    let kvd_byte_offset = dfd_byte_offset + dfd_byte_length;
    let level_byte_offset = kvd_byte_offset as u64;
    let level_byte_length = pixel.len() as u64;

    let header = ktx2::Header {
        format: Some(format),
        type_size,
        pixel_width: 1,
        pixel_height: 1,
        pixel_depth: 0,
        layer_count: 0,
        face_count: 1,
        level_count: 1,
        supercompression_scheme: None,
        index: ktx2::Index { dfd_byte_offset, dfd_byte_length, kvd_byte_offset, kvd_byte_length: 0, sgd_byte_offset: 0, sgd_byte_length: 0 },
    };
    let level_index =
        ktx2::LevelIndex { byte_offset: level_byte_offset, byte_length: level_byte_length, uncompressed_byte_length: level_byte_length };

    let mut out = Vec::with_capacity(level_byte_offset as usize + pixel.len());
    out.extend_from_slice(&header.as_bytes());
    out.extend_from_slice(&level_index.as_bytes());
    out.extend_from_slice(&dfd_byte_length.to_le_bytes());
    out.extend_from_slice(&block_bytes);
    out.extend_from_slice(&pixel);
    out
}

#[test]
fn decode_rgba8_srgb_1x1() {
    let bytes = make_rgba8_srgb_1x1([255, 0, 0, 255]);
    let upload = decode_memory(&bytes).unwrap();
    assert_eq!(upload.desc.width, 1);
    assert_eq!(upload.desc.height, 1);
    assert_eq!(upload.desc.format, TextureFormat::Rgba8UnormSrgb);
    assert_eq!(upload.mipmap, MipmapPolicy::None);
    assert_eq!(upload.data.bytes.as_ref(), &[255, 0, 0, 255]);
}

#[test]
fn reject_garbage() {
    assert!(decode_memory(b"not-a-ktx2").is_err());
}

#[test]
fn reject_undefined_vk_format() {
    // 魔数正确但其余为零：`format = None`（UNDEFINED）→ 拒收。
    let mut bytes = vec![0u8; 128];
    bytes[..12].copy_from_slice(&ktx2::MAGIC);
    assert!(decode_memory(&bytes).is_err());
}
