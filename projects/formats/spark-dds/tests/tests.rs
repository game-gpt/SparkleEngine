//! DDS → TextureUpload 烟测（手写 DXT1 / DX10 RGBA8 夹具）。

use spark_dds::decode_memory;
use spark_texture::{MipmapPolicy, TextureFormat};

fn write_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// 4×4 DXT1（1 block = 8 bytes）。
fn make_dxt1_4x4() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"DDS ");
    write_u32(&mut out, 124); // dwSize
    // CAPS|HEIGHT|WIDTH|PIXELFORMAT|LINEARSIZE
    write_u32(&mut out, 0x1 | 0x2 | 0x4 | 0x1000 | 0x80000);
    write_u32(&mut out, 4); // height
    write_u32(&mut out, 4); // width
    write_u32(&mut out, 8); // pitchOrLinearSize
    write_u32(&mut out, 0); // depth
    write_u32(&mut out, 1); // mipMapCount
    for _ in 0..11 {
        write_u32(&mut out, 0);
    }
    // pixel format
    write_u32(&mut out, 32);
    write_u32(&mut out, 0x4); // DDPF_FOURCC
    write_u32(&mut out, u32::from_le_bytes(*b"DXT1"));
    write_u32(&mut out, 0);
    write_u32(&mut out, 0);
    write_u32(&mut out, 0);
    write_u32(&mut out, 0);
    write_u32(&mut out, 0);
    // caps
    write_u32(&mut out, 0x1000); // DDSCAPS_TEXTURE
    write_u32(&mut out, 0);
    write_u32(&mut out, 0);
    write_u32(&mut out, 0);
    write_u32(&mut out, 0);
    assert_eq!(out.len(), 4 + 124);
    // 1 BC1 block
    out.extend_from_slice(&[0x00, 0xF8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    out
}

/// 1×1 DX10 R8G8B8A8_UNORM_SRGB。
fn make_dx10_rgba8_srgb_1x1(pixel: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"DDS ");
    write_u32(&mut out, 124);
    write_u32(&mut out, 0x1 | 0x2 | 0x4 | 0x1000 | 0x80000);
    write_u32(&mut out, 1);
    write_u32(&mut out, 1);
    write_u32(&mut out, 4);
    write_u32(&mut out, 0);
    write_u32(&mut out, 1);
    for _ in 0..11 {
        write_u32(&mut out, 0);
    }
    write_u32(&mut out, 32);
    write_u32(&mut out, 0x4);
    write_u32(&mut out, u32::from_le_bytes(*b"DX10"));
    write_u32(&mut out, 0);
    write_u32(&mut out, 0);
    write_u32(&mut out, 0);
    write_u32(&mut out, 0);
    write_u32(&mut out, 0);
    write_u32(&mut out, 0x1000);
    write_u32(&mut out, 0);
    write_u32(&mut out, 0);
    write_u32(&mut out, 0);
    write_u32(&mut out, 0);
    // DX10 header
    write_u32(&mut out, 29); // R8G8B8A8_UNORM_SRGB
    write_u32(&mut out, 3); // TEXTURE2D
    write_u32(&mut out, 0); // misc
    write_u32(&mut out, 1); // arraySize
    write_u32(&mut out, 0); // miscFlags2
    out.extend_from_slice(&pixel);
    out
}

#[test]
fn decode_dxt1_4x4() {
    let upload = decode_memory(&make_dxt1_4x4()).unwrap();
    assert_eq!(upload.desc.width, 4);
    assert_eq!(upload.desc.height, 4);
    assert_eq!(upload.desc.format, TextureFormat::Bc1RgbaUnorm);
    assert_eq!(upload.mipmap, MipmapPolicy::None);
    assert_eq!(upload.data.bytes.len(), 8);
}

#[test]
fn decode_dx10_rgba8_srgb() {
    let upload = decode_memory(&make_dx10_rgba8_srgb_1x1([1, 2, 3, 4])).unwrap();
    assert_eq!(upload.desc.format, TextureFormat::Rgba8UnormSrgb);
    assert_eq!(upload.data.bytes.as_ref(), &[1, 2, 3, 4]);
}

#[test]
fn reject_garbage() {
    assert!(decode_memory(b"not-dds").is_err());
}
