//! 将 [`spark_texture::TextureUpload`] 落到 `wgpu::Texture`。

use std::sync::Arc;

use spark_texture::{DeviceCaps, MipmapPolicy, TextureDimension, TextureFormat, TextureUpload, TextureUsage};
use spark_types::{ErrorArg, SparkError, codes};

use crate::mipmap::{create_rgba_texture_with_mips, mip_level_count};

/// 从 `wgpu::Adapter` 探测 [`DeviceCaps`]（压缩族 / 数组 / 尺寸上限）。
pub fn device_caps_from_adapter(adapter: &wgpu::Adapter) -> DeviceCaps {
    let features = adapter.features();
    let limits = adapter.limits();
    DeviceCaps {
        supports_bc: features.contains(wgpu::Features::TEXTURE_COMPRESSION_BC),
        supports_etc2: features.contains(wgpu::Features::TEXTURE_COMPRESSION_ETC2),
        supports_astc: features.contains(wgpu::Features::TEXTURE_COMPRESSION_ASTC),
        // WebGPU 核心含 `rgba16float` 纹理格式；半精度滤波另议。
        supports_float16: true,
        supports_texture_arrays: limits.max_texture_array_layers > 1,
        // GPU mip 生成未接；当前靠 CPU 盒式。
        supports_mip_generation: false,
        max_texture_dimension: limits.max_texture_dimension_2d,
    }
}

/// 将 Spark 纹理格式映射为 wgpu 格式。
pub fn map_texture_format(format: TextureFormat) -> Result<wgpu::TextureFormat, SparkError> {
    Ok(match format {
        TextureFormat::R8Unorm => wgpu::TextureFormat::R8Unorm,
        TextureFormat::Rg8Unorm => wgpu::TextureFormat::Rg8Unorm,
        TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        TextureFormat::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
        TextureFormat::Rgba32Float => wgpu::TextureFormat::Rgba32Float,
        TextureFormat::Bc1RgbaUnorm => wgpu::TextureFormat::Bc1RgbaUnorm,
        TextureFormat::Bc1RgbaUnormSrgb => wgpu::TextureFormat::Bc1RgbaUnormSrgb,
        TextureFormat::Bc3RgbaUnorm => wgpu::TextureFormat::Bc3RgbaUnorm,
        TextureFormat::Bc3RgbaUnormSrgb => wgpu::TextureFormat::Bc3RgbaUnormSrgb,
        TextureFormat::Bc5RgUnorm => wgpu::TextureFormat::Bc5RgUnorm,
        TextureFormat::Bc7RgbaUnorm => wgpu::TextureFormat::Bc7RgbaUnorm,
        TextureFormat::Bc7RgbaUnormSrgb => wgpu::TextureFormat::Bc7RgbaUnormSrgb,
        TextureFormat::Etc2Rgba8Unorm => wgpu::TextureFormat::Etc2Rgba8Unorm,
        TextureFormat::Etc2Rgba8UnormSrgb => wgpu::TextureFormat::Etc2Rgba8UnormSrgb,
        TextureFormat::Astc4x4Unorm => wgpu::TextureFormat::Astc { block: wgpu::AstcBlock::B4x4, channel: wgpu::AstcChannel::Unorm },
        TextureFormat::Astc4x4UnormSrgb => wgpu::TextureFormat::Astc { block: wgpu::AstcBlock::B4x4, channel: wgpu::AstcChannel::UnormSrgb },
        TextureFormat::Astc6x6Unorm => wgpu::TextureFormat::Astc { block: wgpu::AstcBlock::B6x6, channel: wgpu::AstcChannel::Unorm },
        TextureFormat::Astc6x6UnormSrgb => wgpu::TextureFormat::Astc { block: wgpu::AstcBlock::B6x6, channel: wgpu::AstcChannel::UnormSrgb },
    })
}

fn map_dimension(dim: TextureDimension) -> wgpu::TextureDimension {
    match dim {
        TextureDimension::D1 => wgpu::TextureDimension::D1,
        TextureDimension::D2 | TextureDimension::Cube => wgpu::TextureDimension::D2,
        TextureDimension::D3 => wgpu::TextureDimension::D3,
    }
}

fn map_usage(usage: TextureUsage) -> wgpu::TextureUsages {
    let mut out = wgpu::TextureUsages::empty();
    if usage.contains(TextureUsage::COPY_SRC) {
        out |= wgpu::TextureUsages::COPY_SRC;
    }
    if usage.contains(TextureUsage::COPY_DST) {
        out |= wgpu::TextureUsages::COPY_DST;
    }
    if usage.contains(TextureUsage::TEXTURE_BINDING) {
        out |= wgpu::TextureUsages::TEXTURE_BINDING;
    }
    if usage.contains(TextureUsage::STORAGE_BINDING) {
        out |= wgpu::TextureUsages::STORAGE_BINDING;
    }
    if usage.contains(TextureUsage::RENDER_ATTACHMENT) {
        out |= wgpu::TextureUsages::RENDER_ATTACHMENT;
    }
    if out.is_empty() {
        // 不变式：采样纹理至少能上传并绑定。
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST
    }
    else {
        out
    }
}

/// 由 [`TextureUpload`] 创建并上传 `wgpu::Texture`。
///
/// - `MipmapPolicy::GenerateCpu`：仅未压缩 RGBA8（线性 / sRGB），走 CPU 盒式 mip。
/// - `None` / `Provided`：按 layout 的 mip 偏移逐级 `write_texture`（含压缩格式）。
pub fn create_texture_from_upload(device: &wgpu::Device, queue: &wgpu::Queue, upload: &TextureUpload) -> Result<wgpu::Texture, SparkError> {
    upload.validate()?;
    create_texture_from_upload_validated(device, queue, upload)
}

/// 先按 [`DeviceCaps`] 校验再上传（压缩格式选型入口）。
pub fn create_texture_from_upload_with_caps(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    upload: &TextureUpload,
    caps: &DeviceCaps,
) -> Result<wgpu::Texture, SparkError> {
    upload.validate_for_device(caps)?;
    create_texture_from_upload_validated(device, queue, upload)
}

fn create_texture_from_upload_validated(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    upload: &TextureUpload,
) -> Result<wgpu::Texture, SparkError> {
    let label = upload.debug_name.as_deref().unwrap_or("spark-texture");
    let wgpu_format = map_texture_format(upload.desc.format)?;

    match upload.mipmap {
        MipmapPolicy::GenerateCpu => {
            if !matches!(upload.desc.format, TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb) {
                return Err(
                    SparkError::new(codes::texture_upload_invalid()).arg("reason", ErrorArg::String(Arc::from("generate_cpu_rgba8_only")))
                );
            }
            Ok(create_rgba_texture_with_mips(
                device,
                queue,
                label,
                upload.desc.width,
                upload.desc.height,
                upload.data.bytes.as_ref(),
                wgpu_format,
            ))
        }
        MipmapPolicy::None | MipmapPolicy::Provided => create_with_explicit_mips(device, queue, label, upload, wgpu_format),
    }
}

fn create_with_explicit_mips(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    upload: &TextureUpload,
    wgpu_format: wgpu::TextureFormat,
) -> Result<wgpu::Texture, SparkError> {
    let desc = &upload.desc;
    let layout = &upload.data.layout;
    let mip_levels = desc.mip_levels.max(1);
    if layout.mip_offsets.len() as u32 != mip_levels {
        return Err(SparkError::new(codes::texture_layout_invalid())
            .arg("mip_offsets", ErrorArg::Unsigned(layout.mip_offsets.len() as u64))
            .arg("mip_levels", ErrorArg::Unsigned(mip_levels as u64)));
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width: desc.width, height: desc.height, depth_or_array_layers: desc.depth_or_layers },
        mip_level_count: mip_levels,
        sample_count: desc.sample_count.max(1),
        dimension: map_dimension(desc.dimension),
        format: wgpu_format,
        usage: map_usage(desc.usage),
        view_formats: &[],
    });

    let (bw, bh) = desc.format.block_extent();
    let bpb = desc.format.bytes_per_block();
    let bytes = upload.data.bytes.as_ref();

    for level in 0..mip_levels {
        let level_w = (desc.width >> level).max(1);
        let level_h = (desc.height >> level).max(1);
        let offset = layout.mip_offsets[level as usize] as usize;
        let blocks_x = level_w.div_ceil(bw);
        let blocks_y = level_h.div_ceil(bh);
        // 非 0 级始终按紧凑块布局推算行距；0 级可用显式 row_pitch。
        let row_pitch = if layout.row_pitch > 0 && level == 0 { layout.row_pitch } else { blocks_x.saturating_mul(bpb) };
        let level_bytes = (row_pitch as usize).saturating_mul(blocks_y as usize).saturating_mul(desc.depth_or_layers as usize);
        if offset.saturating_add(level_bytes) > bytes.len() {
            return Err(SparkError::new(codes::texture_data_length_mismatch())
                .arg("mip_level", ErrorArg::Unsigned(level as u64))
                .arg("offset", ErrorArg::Unsigned(offset as u64))
                .arg("need", ErrorArg::Unsigned(level_bytes as u64))
                .arg("got", ErrorArg::Unsigned(bytes.len().saturating_sub(offset) as u64)));
        }
        let slice = &bytes[offset..offset + level_bytes];
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: level, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            slice,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row_pitch), rows_per_image: Some(blocks_y) },
            wgpu::Extent3d { width: level_w, height: level_h, depth_or_array_layers: desc.depth_or_layers },
        );
    }
    Ok(texture)
}

/// `GenerateCpu` 时的期望 mip 数，否则为描述中的 `mip_levels`。
pub fn expected_mip_levels(upload: &TextureUpload) -> u32 {
    match upload.mipmap {
        MipmapPolicy::GenerateCpu => mip_level_count(upload.desc.width, upload.desc.height),
        MipmapPolicy::None | MipmapPolicy::Provided => upload.desc.mip_levels.max(1),
    }
}
