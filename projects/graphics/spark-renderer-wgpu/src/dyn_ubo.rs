//! 动态 uniform 槽：同帧多 draw 不得反复 `write_buffer` 覆盖同一偏移。

use std::num::NonZeroU64;

/// 按设备 `min_uniform_buffer_offset_alignment` 对齐的槽步长。
pub fn uniform_stride(device: &wgpu::Device, element_size: u64) -> u64 {
    let align = u64::from(device.limits().min_uniform_buffer_offset_alignment.max(16));
    element_size.div_ceil(align) * align
}

pub fn binding_size(element_size: u64) -> Option<NonZeroU64> {
    NonZeroU64::new(element_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::{Pod, Zeroable};

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct DummyUbo {
        model: [[f32; 4]; 4],
    }

    fn test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::PRIMARY;
        let instance = wgpu::Instance::new(instance_desc);
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("dyn-ubo-smoke"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: Default::default(),
            experimental_features: Default::default(),
            trace: Default::default(),
        }))
        .ok()?;
        Some((device, queue))
    }

    fn make_dyn_bg(
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        element: u64,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dyn-ubo-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: binding_size(element),
                },
                count: None,
            }],
        });
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dyn-ubo-bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer,
                    offset: 0,
                    size: binding_size(element),
                }),
            }],
        });
        (bgl, bg)
    }

    #[test]
    fn dynamic_offset_bind_group_submits() {
        let Some((device, queue)) = test_device() else {
            eprintln!("skip: no wgpu adapter");
            return;
        };
        let element = std::mem::size_of::<DummyUbo>() as u64;
        let stride = uniform_stride(&device, element);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dyn-ubo-ring"),
            size: stride * 4,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (_bgl, bg) = make_dyn_bg(&device, &buffer, element);
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("dyn-ubo-rt"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        let ubo = DummyUbo {
            model: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        };
        queue.write_buffer(&buffer, stride, bytemuck::bytes_of(&ubo));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("dyn-ubo-enc"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("dyn-ubo-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            // 正确：layout 需要 1 个动态偏移。
            pass.set_bind_group(0, &bg, &[stride as u32]);
        }
        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    }

    #[test]
    fn empty_dynamic_offsets_are_rejected() {
        let Some((device, queue)) = test_device() else {
            eprintln!("skip: no wgpu adapter");
            return;
        };
        let element = std::mem::size_of::<DummyUbo>() as u64;
        let stride = uniform_stride(&device, element);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dyn-ubo-ring-bad"),
            size: stride * 2,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (_bgl, bg) = make_dyn_bg(&device, &buffer, element);
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("dyn-ubo-rt-bad"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());

        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("dyn-ubo-bad-enc"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("dyn-ubo-bad-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            // 回归：空动态偏移必须被校验拒绝（此前 mesh3d-bg 崩溃根因）。
            pass.set_bind_group(0, &bg, &[]);
        }
        queue.submit(Some(encoder.finish()));
        let err = pollster::block_on(scope.pop());
        assert!(
            err.is_some(),
            "expected validation error for empty dynamic offsets"
        );
    }
}
