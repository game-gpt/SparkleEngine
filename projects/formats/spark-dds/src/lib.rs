//! DDS → [`TextureUpload`]。
//!
//! 自研最小解析（无 umbrella `image` / `*-sys`）。首切支持：
//! - 传统 FourCC：`DXT1` / `DXT5`
//! - DX10：`BC1` / `BC3` / `BC5` / `BC7`（UNORM / SRGB）与 `R8G8B8A8_UNORM` / `_SRGB`
//! - 未压缩 32-bit RGBA/BGRA（带 alpha 掩码）按字节原样当作 [`TextureFormat::Rgba8Unorm`]
//!
//! 不支持：体积纹理、立方体、纹理数组、复杂 mip 链之外的奇异标志（多 mip 按连续块数据读取）。
//!
//! 错误：`io`（读盘）、`image_decode`（魔数 / 截断 / 头字段）、`texture_size_invalid`（零尺寸）、
//! `texture_format_unsupported`（未映射 FourCC / DXGI）、`texture_upload_invalid`（非 2D / 数组）、
//! `texture_data_length_mismatch` / `image_dimension_overflow`（mip 载荷不足或尺寸溢出）。

#![deny(missing_docs)]

use std::{path::Path, sync::Arc};

use spark_types::{ErrorArg, SparkError, codes};
use spark_texture::{
    AlphaMode, ColorSpace, CpuCopyPolicy, MipmapPolicy, Residency, TextureData, TextureDesc, TextureDimension, TextureFormat,
    TextureLayout, TextureUpload, TextureUsage, UploadPolicy,
};

const MAGIC: &[u8; 4] = b"DDS ";
const DDS_HEADER_SIZE: usize = 124;
const DDS_PIXELFORMAT_SIZE: u32 = 32;
const DDSD_MIPMAPCOUNT: u32 = 0x20000;
const DDPF_FOURCC: u32 = 0x4;
const DDPF_RGB: u32 = 0x40;
const DDPF_ALPHAPIXELS: u32 = 0x1;
const FOURCC_DX10: u32 = u32::from_le_bytes(*b"DX10");
const FOURCC_DXT1: u32 = u32::from_le_bytes(*b"DXT1");
const FOURCC_DXT5: u32 = u32::from_le_bytes(*b"DXT5");

// DXGI_FORMAT 子集
const DXGI_R8G8B8A8_UNORM: u32 = 28;
const DXGI_R8G8B8A8_UNORM_SRGB: u32 = 29;
const DXGI_BC1_UNORM: u32 = 71;
const DXGI_BC1_UNORM_SRGB: u32 = 72;
const DXGI_BC3_UNORM: u32 = 77;
const DXGI_BC3_UNORM_SRGB: u32 = 78;
const DXGI_BC5_UNORM: u32 = 83;
const DXGI_BC7_UNORM: u32 = 98;
const DXGI_BC7_UNORM_SRGB: u32 = 99;

/// 从路径读取 DDS 文件并解码为可上传包。
///
/// 读盘失败 → `io`（附 `path` / `op=read`）；解析失败透传 [`decode_memory`] 的错误码并附加 `path`。
pub fn decode_path(path: impl AsRef<Path>) -> Result<TextureUpload, SparkError> {
    let path = path.as_ref();
    let path_arg = ErrorArg::Path(Arc::from(path.to_string_lossy().as_ref()));
    let bytes = std::fs::read(path).map_err(|e| {
        SparkError::new(codes::io()).arg("path", path_arg.clone()).arg("op", ErrorArg::String(Arc::from("read"))).caused_by(e)
    })?;
    decode_memory(&bytes).map_err(|e| e.arg("path", path_arg))
}

/// 从内存解析 DDS → [`TextureUpload`]（2D、单层、可含连续 mip）。
///
/// 不变式：返回包已 `validate()`；`mipmap` 在 `mip_map_count > 1` 时为 [`MipmapPolicy::Provided`]，否则 [`MipmapPolicy::None`]。
/// 宽高单位为像素；压缩格式按块对齐裁切各级长度。
pub fn decode_memory(bytes: &[u8]) -> Result<TextureUpload, SparkError> {
    if bytes.len() < 4 + DDS_HEADER_SIZE {
        return Err(decode_err("truncated", bytes.len()));
    }
    if &bytes[0..4] != MAGIC {
        return Err(decode_err("bad_magic", bytes.len()));
    }
    let header = &bytes[4..4 + DDS_HEADER_SIZE];
    let dw_size = read_u32(header, 0)?;
    if dw_size != DDS_HEADER_SIZE as u32 {
        return Err(decode_err("bad_header_size", dw_size as usize));
    }
    let flags = read_u32(header, 4)?;
    let height = read_u32(header, 8)?;
    let width = read_u32(header, 12)?;
    if width == 0 || height == 0 {
        return Err(SparkError::new(codes::texture_size_invalid())
            .arg("width", ErrorArg::Unsigned(width as u64))
            .arg("height", ErrorArg::Unsigned(height as u64)));
    }
    let mip_map_count = {
        let n = read_u32(header, 24)?;
        if flags & DDSD_MIPMAPCOUNT != 0 { n.max(1) } else { 1 }
    };

    let pf = &header[72..104];
    let pf_size = read_u32(pf, 0)?;
    if pf_size != DDS_PIXELFORMAT_SIZE {
        return Err(decode_err("bad_pixelformat_size", pf_size as usize));
    }
    let pf_flags = read_u32(pf, 4)?;
    let four_cc = read_u32(pf, 8)?;

    let mut data_offset = 4 + DDS_HEADER_SIZE;
    let (format, srgb) = if pf_flags & DDPF_FOURCC != 0 {
        if four_cc == FOURCC_DX10 {
            if bytes.len() < data_offset + 20 {
                return Err(decode_err("truncated_dx10", bytes.len()));
            }
            let dxgi = read_u32(&bytes[data_offset..], 0)?;
            let resource_dimension = read_u32(&bytes[data_offset..], 4)?;
            // 3 = TEXTURE2D
            if resource_dimension != 3 {
                return Err(SparkError::new(codes::texture_upload_invalid())
                    .arg("format", ErrorArg::String(Arc::from("dds")))
                    .arg("reason", ErrorArg::String(Arc::from("only_texture2d")))
                    .arg("resource_dimension", ErrorArg::Unsigned(resource_dimension as u64)));
            }
            let array_size = read_u32(&bytes[data_offset..], 12)?;
            if array_size > 1 {
                return Err(SparkError::new(codes::texture_upload_invalid())
                    .arg("format", ErrorArg::String(Arc::from("dds")))
                    .arg("reason", ErrorArg::String(Arc::from("texture_array_unsupported")))
                    .arg("array_size", ErrorArg::Unsigned(array_size as u64)));
            }
            data_offset += 20;
            map_dxgi(dxgi)?
        } else {
            map_four_cc(four_cc)?
        }
    } else if pf_flags & DDPF_RGB != 0 {
        // 仅接受 32-bit BGRA/RGBA 掩码常见布局 → 当作 RGBA8 上传（字节原样）。
        let rgb_bit_count = read_u32(pf, 12)?;
        if rgb_bit_count != 32 || pf_flags & DDPF_ALPHAPIXELS == 0 {
            return Err(SparkError::new(codes::texture_format_unsupported())
                .arg("format", ErrorArg::String(Arc::from("dds")))
                .arg("reason", ErrorArg::String(Arc::from("uncompressed_must_be_32bit_rgba"))));
        }
        (TextureFormat::Rgba8Unorm, false)
    } else {
        return Err(SparkError::new(codes::texture_format_unsupported())
            .arg("format", ErrorArg::String(Arc::from("dds")))
            .arg("reason", ErrorArg::String(Arc::from("unknown_pixelformat"))));
    };

    let payload = bytes.get(data_offset..).ok_or_else(|| decode_err("truncated_payload", bytes.len()))?;
    let (bw, bh) = format.block_extent();
    let bpb = format.bytes_per_block();
    let mut packed = Vec::new();
    let mut mip_offsets = Vec::with_capacity(mip_map_count as usize);
    let mut level_w = width;
    let mut level_h = height;
    let mut cursor = 0usize;
    for _ in 0..mip_map_count {
        mip_offsets.push(packed.len() as u64);
        let blocks_x = level_w.div_ceil(bw) as usize;
        let blocks_y = level_h.div_ceil(bh) as usize;
        let level_bytes = blocks_x
            .checked_mul(blocks_y)
            .and_then(|n| n.checked_mul(bpb as usize))
            .ok_or_else(|| {
                SparkError::new(codes::image_dimension_overflow())
                    .arg("width", ErrorArg::Unsigned(level_w as u64))
                    .arg("height", ErrorArg::Unsigned(level_h as u64))
            })?;
        if cursor.saturating_add(level_bytes) > payload.len() {
            return Err(SparkError::new(codes::texture_data_length_mismatch())
                .arg("expected", ErrorArg::Unsigned(level_bytes as u64))
                .arg("got", ErrorArg::Unsigned(payload.len().saturating_sub(cursor) as u64)));
        }
        packed.extend_from_slice(&payload[cursor..cursor + level_bytes]);
        cursor += level_bytes;
        level_w = (level_w / 2).max(1);
        level_h = (level_h / 2).max(1);
    }

    let layout = TextureLayout {
        row_pitch: width.div_ceil(bw).saturating_mul(bpb),
        slice_pitch: 0,
        mip_offsets,
        layer_offsets: vec![0],
        block_width: bw,
        block_height: bh,
        bytes_per_block: bpb,
    };
    let desc = TextureDesc {
        dimension: TextureDimension::D2,
        width,
        height,
        depth_or_layers: 1,
        mip_levels: mip_map_count,
        sample_count: 1,
        format,
        color_space: if srgb || format.is_srgb() { ColorSpace::Srgb } else { ColorSpace::Linear },
        alpha_mode: AlphaMode::Blend,
        usage: TextureUsage::sampled(),
    };
    desc.validate()?;
    let data = TextureData { layout, bytes: Arc::from(packed) };
    data.validate_against(&desc)?;
    let upload = TextureUpload {
        desc,
        data,
        upload_policy: UploadPolicy::Immediate,
        cpu_copy: CpuCopyPolicy::Discard,
        residency: Residency::Resident,
        mipmap: if mip_map_count > 1 { MipmapPolicy::Provided } else { MipmapPolicy::None },
        debug_name: None,
    };
    upload.validate()?;
    Ok(upload)
}

fn map_four_cc(four_cc: u32) -> Result<(TextureFormat, bool), SparkError> {
    match four_cc {
        FOURCC_DXT1 => Ok((TextureFormat::Bc1RgbaUnorm, false)),
        FOURCC_DXT5 => Ok((TextureFormat::Bc3RgbaUnorm, false)),
        _ => Err(SparkError::new(codes::texture_format_unsupported())
            .arg("format", ErrorArg::String(Arc::from("dds")))
            .arg("four_cc", ErrorArg::Unsigned(four_cc as u64))),
    }
}

fn map_dxgi(dxgi: u32) -> Result<(TextureFormat, bool), SparkError> {
    let mapped = match dxgi {
        DXGI_R8G8B8A8_UNORM => Some((TextureFormat::Rgba8Unorm, false)),
        DXGI_R8G8B8A8_UNORM_SRGB => Some((TextureFormat::Rgba8UnormSrgb, true)),
        DXGI_BC1_UNORM => Some((TextureFormat::Bc1RgbaUnorm, false)),
        DXGI_BC1_UNORM_SRGB => Some((TextureFormat::Bc1RgbaUnormSrgb, true)),
        DXGI_BC3_UNORM => Some((TextureFormat::Bc3RgbaUnorm, false)),
        DXGI_BC3_UNORM_SRGB => Some((TextureFormat::Bc3RgbaUnormSrgb, true)),
        DXGI_BC5_UNORM => Some((TextureFormat::Bc5RgUnorm, false)),
        DXGI_BC7_UNORM => Some((TextureFormat::Bc7RgbaUnorm, false)),
        DXGI_BC7_UNORM_SRGB => Some((TextureFormat::Bc7RgbaUnormSrgb, true)),
        _ => None,
    };
    mapped.ok_or_else(|| {
        SparkError::new(codes::texture_format_unsupported())
            .arg("format", ErrorArg::String(Arc::from("dds")))
            .arg("dxgi_format", ErrorArg::Unsigned(dxgi as u64))
    })
}

fn read_u32(buf: &[u8], offset: usize) -> Result<u32, SparkError> {
    let b = buf.get(offset..offset + 4).ok_or_else(|| decode_err("truncated_u32", offset))?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn decode_err(reason: &'static str, detail: usize) -> SparkError {
    SparkError::new(codes::image_decode())
        .arg("format", ErrorArg::String(Arc::from("dds")))
        .arg("reason", ErrorArg::String(Arc::from(reason)))
        .arg("detail", ErrorArg::Unsigned(detail as u64))
}
