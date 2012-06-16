//! 简易全屏 bloom：场景 RT → 提取 → 可分模糊 → 合成到交换链。

use bytemuck::{Pod, Zeroable};
use spark_shader::{create_builtin, BuiltinShader};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BlurUniforms {
    direction: [f32; 2],
    strength: f32,
    _pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CompUniforms {
    strength: f32,
    _pad: [f32; 3],
}

struct Rt {
    /// 持有 GPU 纹理生命周期；视图 / bind 引用其上。
    #[allow(dead_code)]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind: wgpu::BindGroup,
}

/// 半分辨率 bloom ping-pong + 全分辨率场景色。
pub struct BloomGpu {
    format: wgpu::TextureFormat,
    extract_pl: wgpu::RenderPipeline,
    blur_pl: wgpu::RenderPipeline,
    composite_pl: wgpu::RenderPipeline,
    filter_bgl: wgpu::BindGroupLayout,
    composite_bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    blur_uniform: wgpu::Buffer,
    comp_uniform: wgpu::Buffer,
    scene: Option<Rt>,
    bloom_a: Option<Rt>,
    bloom_b: Option<Rt>,
    /// 合成 bind group：随 RT 重建，普通帧复用。
    composite_bind: Option<wgpu::BindGroup>,
    width: u32,
    height: u32,
}

impl BloomGpu {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = create_builtin(device, BuiltinShader::Bloom);
        let comp_shader = create_builtin(device, BuiltinShader::BloomComposite);

        let filter_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom-filter-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let composite_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom-comp-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let filter_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom-filter-pl"),
            bind_group_layouts: &[Some(&filter_bgl)],
            immediate_size: 0,
        });
        let composite_pl_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom-comp-pl"),
            bind_group_layouts: &[Some(&composite_bgl)],
            immediate_size: 0,
        });

        let extract_pl = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom-extract"),
            layout: Some(&filter_pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_extract"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let blur_pl = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom-blur"),
            layout: Some(&filter_pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_blur"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let composite_pl = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom-composite"),
            layout: Some(&composite_pl_layout),
            vertex: wgpu::VertexState {
                module: &comp_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &comp_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let blur_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bloom-blur-uniform"),
            size: std::mem::size_of::<BlurUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let comp_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bloom-comp-uniform"),
            size: std::mem::size_of::<CompUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            format,
            extract_pl,
            blur_pl,
            composite_pl,
            filter_bgl,
            composite_bgl,
            sampler,
            blur_uniform,
            comp_uniform,
            scene: None,
            bloom_a: None,
            bloom_b: None,
            composite_bind: None,
            width: 0,
            height: 0,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if self.width == width && self.height == height && self.scene.is_some() {
            return;
        }
        self.width = width;
        self.height = height;
        let hw = (width / 2).max(1);
        let hh = (height / 2).max(1);
        self.scene = Some(self.make_rt(device, width, height, "bloom-scene"));
        self.bloom_a = Some(self.make_rt(device, hw, hh, "bloom-a"));
        self.bloom_b = Some(self.make_rt(device, hw, hh, "bloom-b"));
        self.rebuild_composite_bind(device);
    }

    fn rebuild_composite_bind(&mut self, device: &wgpu::Device) {
        let Some(scene) = self.scene.as_ref() else {
            self.composite_bind = None;
            return;
        };
        let Some(a) = self.bloom_a.as_ref() else {
            self.composite_bind = None;
            return;
        };
        self.composite_bind = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom-comp-bg"),
            layout: &self.composite_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&scene.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&a.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.comp_uniform.as_entire_binding(),
                },
            ],
        }));
    }

    fn make_rt(&self, device: &wgpu::Device, w: u32, h: u32, label: &str) -> Rt {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{label}-bg")),
            layout: &self.filter_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.blur_uniform.as_entire_binding(),
                },
            ],
        });
        Rt {
            texture,
            view,
            bind,
        }
    }

    pub fn scene_view(&self) -> Option<&wgpu::TextureView> {
        self.scene.as_ref().map(|r| &r.view)
    }

    /// 从场景色提取模糊并合成到 `dst`（通常为交换链）。
    pub fn apply(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        dst: &wgpu::TextureView,
        strength: f32,
    ) {
        let Some(scene) = self.scene.as_ref() else {
            return;
        };
        let Some(a) = self.bloom_a.as_ref() else {
            return;
        };
        let Some(b) = self.bloom_b.as_ref() else {
            return;
        };
        let Some(comp_bind) = self.composite_bind.as_ref() else {
            return;
        };

        // Extract: scene → A
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom-extract"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &a.view,
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
            pass.set_pipeline(&self.extract_pl);
            pass.set_bind_group(0, &scene.bind, &[]);
            pass.draw(0..3, 0..1);
        }

        // Blur H: A → B
        queue.write_buffer(
            &self.blur_uniform,
            0,
            bytemuck::bytes_of(&BlurUniforms {
                direction: [1.0, 0.0],
                strength,
                _pad: 0.0,
            }),
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom-blur-h"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &b.view,
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
            pass.set_pipeline(&self.blur_pl);
            pass.set_bind_group(0, &a.bind, &[]);
            pass.draw(0..3, 0..1);
        }

        // Blur V: B → A
        queue.write_buffer(
            &self.blur_uniform,
            0,
            bytemuck::bytes_of(&BlurUniforms {
                direction: [0.0, 1.0],
                strength,
                _pad: 0.0,
            }),
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom-blur-v"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &a.view,
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
            pass.set_pipeline(&self.blur_pl);
            pass.set_bind_group(0, &b.bind, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.write_buffer(
            &self.comp_uniform,
            0,
            bytemuck::bytes_of(&CompUniforms {
                strength: strength.clamp(0.0, 2.0),
                _pad: [0.0; 3],
            }),
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom-composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: dst,
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
            pass.set_pipeline(&self.composite_pl);
            pass.set_bind_group(0, comp_bind, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
