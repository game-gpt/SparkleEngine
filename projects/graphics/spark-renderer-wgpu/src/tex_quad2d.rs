//! 2D 屏幕空间 RGBA 纹理四边形（像素图集 / 图标）。

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use spark_core::SparkError;
use spark_renderer::{DrawList, RgbaImage, TexQuadCmd, TextureId};
use spark_shader::{create_builtin, BuiltinShader};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TexQuadVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

struct GpuTex {
    _texture: wgpu::Texture,
    bind: wgpu::BindGroup,
}

/// 2D 纹理四边形管线与缓存。
pub struct TexQuad2dGpu {
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    textures: HashMap<u32, GpuTex>,
    vbo: wgpu::Buffer,
    cap: u64,
    /// 本帧批：`(texture_id, vertex_start, vertex_count)`。
    frame_ranges: Vec<(u32, u32, u32)>,
}

impl TexQuad2dGpu {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        uniform_buf: &wgpu::Buffer,
    ) -> Self {
        let shader = create_builtin(device, BuiltinShader::TexturedQuad);
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tex-quad2d-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
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
            ],
        });
        // 像素风格默认近邻采样。
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("tex-quad2d-nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tex-quad2d-pl"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tex-quad2d-pipeline"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some(BuiltinShader::TexturedQuad.vertex_entry()),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TexQuadVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(BuiltinShader::TexturedQuad.fragment_entry()),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let _ = uniform_buf;
        let cap = 16384u64;
        let vbo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tex-quad2d-vbo"),
            size: cap * std::mem::size_of::<TexQuadVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            bgl,
            sampler,
            textures: HashMap::new(),
            vbo,
            cap,
            frame_ranges: Vec::new(),
        }
    }

    pub fn ingest_uploads(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uniform_buf: &wgpu::Buffer,
        uploads: &[(TextureId, RgbaImage)],
    ) -> Result<(), SparkError> {
        for (id, img) in uploads {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("tex-quad2d"),
                size: wgpu::Extent3d {
                    width: img.width,
                    height: img.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &img.rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(img.width * 4),
                    rows_per_image: Some(img.height),
                },
                wgpu::Extent3d {
                    width: img.width,
                    height: img.height,
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&Default::default());
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tex-quad2d-bg"),
                layout: &self.bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            self.textures.insert(
                id.0,
                GpuTex {
                    _texture: texture,
                    bind,
                },
            );
        }
        Ok(())
    }

    fn ensure_cap(&mut self, device: &wgpu::Device, need: u64) {
        if need <= self.cap {
            return;
        }
        let mut cap = self.cap.max(4096);
        while cap < need {
            cap = cap.saturating_mul(2);
        }
        self.vbo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tex-quad2d-vbo"),
            size: cap * std::mem::size_of::<TexQuadVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.cap = cap;
    }

    /// 上传顶点并记下本帧批范围（须在开 pass 前调用）。
    pub fn prepare_draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cmds: &[TexQuadCmd],
    ) {
        self.frame_ranges.clear();
        if cmds.is_empty() {
            return;
        }
        let mut order: Vec<u32> = Vec::new();
        let mut groups: HashMap<u32, Vec<&TexQuadCmd>> = HashMap::new();
        for c in cmds {
            groups.entry(c.texture.0).or_default().push(c);
            if !order.contains(&c.texture.0) {
                order.push(c.texture.0);
            }
        }
        let mut all_verts: Vec<TexQuadVertex> = Vec::new();
        for id in order {
            let Some(batch) = groups.get(&id) else {
                continue;
            };
            if !self.textures.contains_key(&id) {
                continue;
            }
            let start = all_verts.len() as u32;
            for q in batch {
                let x0 = q.dest.x;
                let y0 = q.dest.y;
                let x1 = q.dest.x + q.dest.w;
                let y1 = q.dest.y + q.dest.h;
                let u0 = q.uv.x;
                let v0 = q.uv.y;
                let u1 = q.uv.x + q.uv.w;
                let v1 = q.uv.y + q.uv.h;
                let c = q.color.to_array();
                all_verts.extend_from_slice(&[
                    TexQuadVertex {
                        pos: [x0, y0],
                        uv: [u0, v0],
                        color: c,
                    },
                    TexQuadVertex {
                        pos: [x1, y0],
                        uv: [u1, v0],
                        color: c,
                    },
                    TexQuadVertex {
                        pos: [x1, y1],
                        uv: [u1, v1],
                        color: c,
                    },
                    TexQuadVertex {
                        pos: [x0, y0],
                        uv: [u0, v0],
                        color: c,
                    },
                    TexQuadVertex {
                        pos: [x1, y1],
                        uv: [u1, v1],
                        color: c,
                    },
                    TexQuadVertex {
                        pos: [x0, y1],
                        uv: [u0, v1],
                        color: c,
                    },
                ]);
            }
            let count = all_verts.len() as u32 - start;
            self.frame_ranges.push((id, start, count));
        }
        if all_verts.is_empty() {
            return;
        }
        self.ensure_cap(device, all_verts.len() as u64);
        queue.write_buffer(&self.vbo, 0, bytemuck::cast_slice(&all_verts));
    }

    /// 在已开启的 pass 中提交本帧纹理四边形（仅共享借用）。
    pub fn encode_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.frame_ranges.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vbo.slice(..));
        for &(id, start, count) in &self.frame_ranges {
            let Some(tex) = self.textures.get(&id) else {
                continue;
            };
            pass.set_bind_group(0, &tex.bind, &[]);
            pass.draw(start..start + count, 0..1);
        }
    }

    pub fn prepare_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uniform_buf: &wgpu::Buffer,
        list: &DrawList,
    ) -> Result<(), SparkError> {
        self.ingest_uploads(device, queue, uniform_buf, &list.texture_uploads)?;
        self.prepare_draw(device, queue, &list.tex_quads);
        Ok(())
    }
}
