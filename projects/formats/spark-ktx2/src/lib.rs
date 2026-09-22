//! KTX2 → [`TextureUpload`]。
//!
//! 使用 pure Rust [`ktx2`] 解析容器。**不**依赖 umbrella `image` / `*-sys`。
//! 首切仅支持无超级压缩（`supercompression_scheme = None`）且 `vkFormat` 可映射的格式。
//! BasisLZ / Zstd / ZLIB 与 `VK_FORMAT_UNDEFINED` 转码后置。

#![warn(missing_docs)]

use std::{path::Path, sync::Arc};

use spark_core::{ErrorArg, SparkError, codes};
use spark_texture::{
    AlphaMode, ColorSpace, CpuCopyPolicy, MipmapPolicy, Residency, TextureData, TextureDesc, TextureDimension, TextureFormat,
    TextureLayout, TextureUpload, TextureUsage, UploadPolicy,
};

/// 从路径读 KTX2。
pub fn decode_path(path: impl AsRef<Path>) -> Result<TextureUpload, SparkError> {
    let path = path.as_ref();
    let path_arg = ErrorArg::Path(Arc::from(path.to_string_lossy().as_ref()));
    let bytes = std::fs::read(path).map_err(|e| {
        SparkError::new(codes::io()).arg("path", path_arg.clone()).arg("op", ErrorArg::String(Arc::from("read"))).caused_by(e)
    })?;
    decode_memory(&bytes).map_err(|e| e.arg("path", path_arg))
}

/// 从内存解析 KTX2 → [`TextureUpload`]。
pub fn decode_memory(bytes: &[u8]) -> Result<TextureUpload, SparkError> {
    let reader = ktx2::Reader::new(bytes).map_err(|e| {
        SparkError::new(codes::image_decode())
            .arg("format", ErrorArg::String(Arc::from("ktx2")))
            .arg("op", ErrorArg::String(Arc::from("parse")))
            .arg("bytes", ErrorArg::Unsigned(bytes.len() as u64))
            .caused_by(e)
    })?;
    let header = reader.header();
    if header.supercompression_scheme.is_some() {
        return Err(SparkError::new(codes::image_decode())
            .arg("format", ErrorArg::String(Arc::from("ktx2")))
            .arg("reason", ErrorArg::String(Arc::from("supercompression_unsupported"))));
    }
    let vk = header.format.ok_or_else(|| {
        SparkError::new(codes::image_decode())
            .arg("format", ErrorArg::String(Arc::from("ktx2")))
            .arg("reason", ErrorArg::String(Arc::from("undefined_vk_format_needs_transcode")))
    })?;
    let tex_format = map_vk_format(vk)?;
    let dimension = map_dimension(&header)?;
    let width = header.pixel_width;
    // 1D：`pixel_height == 0`；描述层要求非零。
    let height = header.pixel_height.max(1);
    let depth_or_layers = map_depth_or_layers(&header)?;
    let levels: Vec<_> = reader.levels().collect();
    if levels.is_empty() {
        return Err(SparkError::new(codes::texture_layout_invalid())
            .arg("mip_offsets", ErrorArg::Unsigned(0))
            .arg("mip_levels", ErrorArg::Unsigned(0)));
    }
    let mip_levels = levels.len() as u32;

    let (bw, bh) = tex_format.block_extent();
    let bpb = tex_format.bytes_per_block();
    let mut packed = Vec::new();
    let mut mip_offsets = Vec::with_capacity(levels.len());
    for level in &levels {
        mip_offsets.push(packed.len() as u64);
        packed.extend_from_slice(level.data);
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
        dimension,
        width,
        height,
        depth_or_layers,
        mip_levels,
        sample_count: 1,
        format: tex_format,
        color_space: if tex_format.is_srgb() { ColorSpace::Srgb } else { ColorSpace::Linear },
        alpha_mode: AlphaMode::Blend,
        usage: TextureUsage::sampled(),
    };
    desc.validate()?;

    let data = TextureData { layout, bytes: Arc::from(packed) };
    data.validate_against(&desc)?;

    // `level_count == 0` 表示容器只存 base，应用可补 mip（仅未压缩可走 CPU 生成）。
    let mipmap = if mip_levels > 1 {
        MipmapPolicy::Provided
    } else if header.level_count == 0 && !tex_format.is_compressed() {
        MipmapPolicy::GenerateCpu
    } else {
        MipmapPolicy::None
    };

    let upload = TextureUpload {
        desc,
        data,
        upload_policy: UploadPolicy::Immediate,
        cpu_copy: CpuCopyPolicy::Discard,
        residency: Residency::Resident,
        mipmap,
        debug_name: None,
    };
    upload.validate()?;
    Ok(upload)
}

fn map_dimension(header: &ktx2::Header) -> Result<TextureDimension, SparkError> {
    if header.face_count == 6 {
        return Ok(TextureDimension::Cube);
    }
    if header.pixel_depth > 0 {
        return Ok(TextureDimension::D3);
    }
    if header.pixel_height == 0 {
        return Ok(TextureDimension::D1);
    }
    Ok(TextureDimension::D2)
}

fn map_depth_or_layers(header: &ktx2::Header) -> Result<u32, SparkError> {
    if header.face_count == 6 {
        // 首切：仅单层立方体（`TextureDesc` 要求 `depth_or_layers == 6`）。
        if header.layer_count > 1 {
            return Err(SparkError::new(codes::texture_upload_invalid())
                .arg("format", ErrorArg::String(Arc::from("ktx2")))
                .arg("reason", ErrorArg::String(Arc::from("cube_array_unsupported")))
                .arg("layer_count", ErrorArg::Unsigned(header.layer_count as u64)));
        }
        return Ok(6);
    }
    if header.pixel_depth > 0 {
        return Ok(header.pixel_depth);
    }
    Ok(header.layer_count.max(1))
}

fn map_vk_format(format: ktx2::Format) -> Result<TextureFormat, SparkError> {
    // 与 spark-texture / wgpu 首切对齐的常用子集。
    let mapped = if format == ktx2::Format::R8_UNORM {
        Some(TextureFormat::R8Unorm)
    } else if format == ktx2::Format::R8G8_UNORM {
        Some(TextureFormat::Rg8Unorm)
    } else if format == ktx2::Format::R8G8B8A8_UNORM {
        Some(TextureFormat::Rgba8Unorm)
    } else if format == ktx2::Format::R8G8B8A8_SRGB {
        Some(TextureFormat::Rgba8UnormSrgb)
    } else if format == ktx2::Format::R16G16B16A16_SFLOAT {
        Some(TextureFormat::Rgba16Float)
    } else if format == ktx2::Format::R32G32B32A32_SFLOAT {
        Some(TextureFormat::Rgba32Float)
    } else if format == ktx2::Format::BC1_RGBA_UNORM_BLOCK {
        Some(TextureFormat::Bc1RgbaUnorm)
    } else if format == ktx2::Format::BC1_RGBA_SRGB_BLOCK {
        Some(TextureFormat::Bc1RgbaUnormSrgb)
    } else if format == ktx2::Format::BC3_UNORM_BLOCK {
        Some(TextureFormat::Bc3RgbaUnorm)
    } else if format == ktx2::Format::BC3_SRGB_BLOCK {
        Some(TextureFormat::Bc3RgbaUnormSrgb)
    } else if format == ktx2::Format::BC5_UNORM_BLOCK {
        Some(TextureFormat::Bc5RgUnorm)
    } else if format == ktx2::Format::BC7_UNORM_BLOCK {
        Some(TextureFormat::Bc7RgbaUnorm)
    } else if format == ktx2::Format::BC7_SRGB_BLOCK {
        Some(TextureFormat::Bc7RgbaUnormSrgb)
    } else if format == ktx2::Format::ETC2_R8G8B8A8_UNORM_BLOCK {
        Some(TextureFormat::Etc2Rgba8Unorm)
    } else if format == ktx2::Format::ETC2_R8G8B8A8_SRGB_BLOCK {
        Some(TextureFormat::Etc2Rgba8UnormSrgb)
    } else if format == ktx2::Format::ASTC_4x4_UNORM_BLOCK {
        Some(TextureFormat::Astc4x4Unorm)
    } else if format == ktx2::Format::ASTC_4x4_SRGB_BLOCK {
        Some(TextureFormat::Astc4x4UnormSrgb)
    } else if format == ktx2::Format::ASTC_6x6_UNORM_BLOCK {
        Some(TextureFormat::Astc6x6Unorm)
    } else if format == ktx2::Format::ASTC_6x6_SRGB_BLOCK {
        Some(TextureFormat::Astc6x6UnormSrgb)
    } else {
        None
    };
    mapped.ok_or_else(|| {
        SparkError::new(codes::texture_format_unsupported())
            .arg("format", ErrorArg::String(Arc::from("ktx2")))
            .arg("vk_format", ErrorArg::Unsigned(format.value() as u64))
    })
}
