//! 2D RGBA 纹理 mip 链：CPU 盒式下采样后逐级 `write_texture`。
//!
//! 高清源图以原尺寸上传时，小屏幕覆盖面积必须走 mip，否则会出现细线闪烁与摩尔纹。

/// `floor(log2(max(w,h))) + 1`，至少 1 级。
pub fn mip_level_count(width: u32, height: u32) -> u32 {
    let max_dim = width.max(height).max(1);
    32 - max_dim.leading_zeros()
}

/// 预乘 alpha 的 2×2 盒式下采样（奇数边取 1 或 2 个源像素）。
pub fn downsample_rgba(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    for dy in 0..dh {
        for dx in 0..dw {
            let x0 = dx * 2;
            let y0 = dy * 2;
            let x1 = (x0 + 1).min(sw - 1);
            let y1 = (y0 + 1).min(sh - 1);
            let mut r = 0.0f32;
            let mut g = 0.0f32;
            let mut b = 0.0f32;
            let mut a = 0.0f32;
            let mut n = 0.0f32;
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let i = ((y * sw + x) * 4) as usize;
                    if i + 3 >= src.len() {
                        continue;
                    }
                    let aa = src[i + 3] as f32 / 255.0;
                    r += src[i] as f32 / 255.0 * aa;
                    g += src[i + 1] as f32 / 255.0 * aa;
                    b += src[i + 2] as f32 / 255.0 * aa;
                    a += aa;
                    n += 1.0;
                }
            }
            let o = ((dy * dw + dx) * 4) as usize;
            if a < 1e-4 || n <= 0.0 {
                out[o] = 0;
                out[o + 1] = 0;
                out[o + 2] = 0;
                out[o + 3] = 0;
            }
            else {
                out[o] = ((r / a) * 255.0).round().clamp(0.0, 255.0) as u8;
                out[o + 1] = ((g / a) * 255.0).round().clamp(0.0, 255.0) as u8;
                out[o + 2] = ((b / a) * 255.0).round().clamp(0.0, 255.0) as u8;
                out[o + 3] = ((a / n) * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

/// 创建带完整 mip 链的 RGBA8 纹理并上传全部层级。
///
/// `format` 应为 `Rgba8Unorm` 或 `Rgba8UnormSrgb`。
pub fn create_rgba_texture_with_mips(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    let levels = mip_level_count(width, height);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: levels,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    let mut level_w = width;
    let mut level_h = height;
    let mut level_rgba = rgba.to_vec();
    for level in 0..levels {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: level, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &level_rgba,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(level_w * 4), rows_per_image: Some(level_h) },
            wgpu::Extent3d { width: level_w, height: level_h, depth_or_array_layers: 1 },
        );
        if level + 1 >= levels {
            break;
        }
        let next_w = (level_w / 2).max(1);
        let next_h = (level_h / 2).max(1);
        level_rgba = downsample_rgba(&level_rgba, level_w, level_h, next_w, next_h);
        level_w = next_w;
        level_h = next_h;
    }
    texture
}
