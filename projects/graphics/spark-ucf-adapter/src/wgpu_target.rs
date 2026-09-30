//! wgpu RGBA8 目标：UCF 结果 staging 上传（device-resident UCF 后置）。

use wgpu::{
    Extent3d, Origin3d, Queue, TexelCopyBufferLayout, TexelCopyTextureInfo, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

/// 可采样 RGBA8 纹理目标（`COPY_DST` + `TEXTURE_BINDING`）。
pub struct WgpuRgba8Target {
    width: u32,
    height: u32,
    texture: Texture,
    view: TextureView,
}

impl WgpuRgba8Target {
    /// 分配空 RGBA8 纹理。
    pub fn new(device: &wgpu::Device, width: u32, height: u32, label: Option<&str>) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        Self {
            width,
            height,
            texture,
            view,
        }
    }

    /// 逻辑宽（像素）。
    pub fn width(&self) -> u32 {
        self.width
    }

    /// 逻辑高（像素）。
    pub fn height(&self) -> u32 {
        self.height
    }

    /// 底层 `wgpu::Texture`（供渲染后端绑定）。
    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    /// 默认视图。
    pub fn view(&self) -> &TextureView {
        &self.view
    }

    /// 将 RGBA8 行主序字节上传到 mip0。
    pub fn upload_rgba8(queue: &Queue, target: &Self, rgba: &[u8]) -> Result<(), String> {
        let w = target.width;
        let h = target.height;
        let pixels = (w as usize).saturating_mul(h as usize);
        let need = pixels * 4;
        if rgba.len() != need {
            return Err(format!(
                "rgba8 upload expected {need} bytes for {w}x{h}, got {}",
                rgba.len()
            ));
        }
        let bytes_per_row = w * 4;
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &target.texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(h),
            },
            Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    }

    /// 读回 mip0 RGBA8（测试 / 调试；生产路径应直接采样纹理）。
    pub fn read_rgba8_back(
        device: &wgpu::Device,
        queue: &Queue,
        target: &Self,
    ) -> Result<Vec<u8>, String> {
        let width = target.width.max(1);
        let height = target.height.max(1);
        let unpadded = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded = unpadded.div_ceil(align) * align;
        let buffer_size = padded as u64 * height as u64;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("spark-ucf-adapter readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("spark-ucf-adapter readback"),
        });
        encoder.copy_texture_to_buffer(
            target.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        let slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = sender.send(r);
        });
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| e.to_string())?;
        receiver
            .recv()
            .map_err(|_| "map_async dropped".to_string())?
            .map_err(|e| e.to_string())?;

        let mapped = slice.get_mapped_range().map_err(|e| e.to_string())?;
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height as usize {
            let start = row * padded as usize;
            let end = start + unpadded as usize;
            out.extend_from_slice(&mapped[start..end]);
        }
        Ok(out)
    }
}
