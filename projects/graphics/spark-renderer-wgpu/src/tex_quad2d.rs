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
    /// 本帧批：`(texture_id, vertex_start, vertex_count)`，保持提交顺序。
    frame_ranges: Vec<(u32, u32, u32)>,
    /// `prepare_layered` 后世界层顶点上界（HUD 纹理从此开始）。
    world_vert_end: u32,
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
        // 近距、缩小时都用最近点。像素图按整数倍放大时，线性过滤会把块面糊掉。
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("tex-quad2d-mip"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
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
            world_vert_end: 0,
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
            let texture = crate::mipmap::create_rgba_texture_with_mips(
                device,
                queue,
                "tex-quad2d",
                img.width,
                img.height,
                &img.rgba,
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

    fn append_quad(verts: &mut Vec<TexQuadVertex>, q: &TexQuadCmd) {
        let x0 = q.dest.x;
        let y0 = q.dest.y;
        let x1 = q.dest.x + q.dest.w;
        let y1 = q.dest.y + q.dest.h;
        let u0 = q.uv.x;
        let v0 = q.uv.y;
        let u1 = q.uv.x + q.uv.w;
        let v1 = q.uv.y + q.uv.h;
        let c = q.color.to_array();
        // 绕 dest 中心旋转屏幕坐标。着色器仍只做像素→NDC。
        let (p00, p10, p11, p01) = if q.angle_rad.abs() < 1e-8 {
            ([x0, y0], [x1, y0], [x1, y1], [x0, y1])
        } else {
            let (s, cos) = q.angle_rad.sin_cos();
            let cx = (x0 + x1) * 0.5;
            let cy = (y0 + y1) * 0.5;
            let rot = |px: f32, py: f32| -> [f32; 2] {
                let dx = px - cx;
                let dy = py - cy;
                [cx + dx * cos - dy * s, cy + dx * s + dy * cos]
            };
            (rot(x0, y0), rot(x1, y0), rot(x1, y1), rot(x0, y1))
        };
        verts.extend_from_slice(&[
            TexQuadVertex {
                pos: p00,
                uv: [u0, v0],
                color: c,
            },
            TexQuadVertex {
                pos: p10,
                uv: [u1, v0],
                color: c,
            },
            TexQuadVertex {
                pos: p11,
                uv: [u1, v1],
                color: c,
            },
            TexQuadVertex {
                pos: p00,
                uv: [u0, v0],
                color: c,
            },
            TexQuadVertex {
                pos: p11,
                uv: [u1, v1],
                color: c,
            },
            TexQuadVertex {
                pos: p01,
                uv: [u0, v1],
                color: c,
            },
        ]);
    }

    /// 按命令顺序上传顶点；仅合并**连续**同纹理批，不重排。
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
        let mut all_verts: Vec<TexQuadVertex> = Vec::new();
        let mut cur_id: Option<u32> = None;
        let mut run_start = 0u32;
        for q in cmds {
            let id = q.texture.0;
            if !self.textures.contains_key(&id) {
                continue;
            }
            if cur_id != Some(id) {
                if let Some(prev) = cur_id {
                    let count = all_verts.len() as u32 - run_start;
                    if count > 0 {
                        self.frame_ranges.push((prev, run_start, count));
                    }
                }
                cur_id = Some(id);
                run_start = all_verts.len() as u32;
            }
            Self::append_quad(&mut all_verts, q);
        }
        if let Some(prev) = cur_id {
            let count = all_verts.len() as u32 - run_start;
            if count > 0 {
                self.frame_ranges.push((prev, run_start, count));
            }
        }
        if all_verts.is_empty() {
            return;
        }
        self.ensure_cap(device, all_verts.len() as u64);
        queue.write_buffer(&self.vbo, 0, bytemuck::cast_slice(&all_verts));
    }

    pub fn encode_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.encode_pass_range(pass, 0, u32::MAX);
    }

    /// 只提交顶点落在 `[vert_lo, vert_hi)` 内的批（用于 world/hud 分层）。
    pub fn encode_pass_range<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        vert_lo: u32,
        vert_hi: u32,
    ) {
        if self.frame_ranges.is_empty() || vert_lo >= vert_hi {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vbo.slice(..));
        for &(id, start, count) in &self.frame_ranges {
            let end = start + count;
            let lo = start.max(vert_lo);
            let hi = end.min(vert_hi);
            if lo >= hi {
                continue;
            }
            let Some(tex) = self.textures.get(&id) else {
                continue;
            };
            pass.set_bind_group(0, &tex.bind, &[]);
            pass.draw(lo..hi, 0..1);
        }
    }

    pub fn world_vert_end(&self) -> u32 {
        self.world_vert_end
    }

    pub fn prepare_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uniform_buf: &wgpu::Buffer,
        list: &DrawList,
    ) -> Result<(), SparkError> {
        self.ingest_uploads(device, queue, uniform_buf, &list.texture_uploads)?;
        self.frame_ranges.clear();
        let mut all_verts: Vec<TexQuadVertex> = Vec::new();

        Self::append_cmds_ordered(
            &self.textures,
            &list.tex_quads,
            &mut all_verts,
            &mut self.frame_ranges,
        );
        self.world_vert_end = all_verts.len() as u32;
        Self::append_cmds_ordered(
            &self.textures,
            &list.hud_tex_quads,
            &mut all_verts,
            &mut self.frame_ranges,
        );

        if !all_verts.is_empty() {
            self.ensure_cap(device, all_verts.len() as u64);
            queue.write_buffer(&self.vbo, 0, bytemuck::cast_slice(&all_verts));
        }
        Ok(())
    }

    fn append_cmds_ordered(
        textures: &HashMap<u32, GpuTex>,
        cmds: &[TexQuadCmd],
        all_verts: &mut Vec<TexQuadVertex>,
        ranges: &mut Vec<(u32, u32, u32)>,
    ) {
        let mut cur_id: Option<u32> = None;
        let mut run_start = 0u32;
        for q in cmds {
            let id = q.texture.0;
            if !textures.contains_key(&id) {
                continue;
            }
            if cur_id != Some(id) {
                if let Some(prev) = cur_id {
                    let count = all_verts.len() as u32 - run_start;
                    if count > 0 {
                        ranges.push((prev, run_start, count));
                    }
                }
                cur_id = Some(id);
                run_start = all_verts.len() as u32;
            }
            Self::append_quad(all_verts, q);
        }
        if let Some(prev) = cur_id {
            let count = all_verts.len() as u32 - run_start;
            if count > 0 {
                ranges.push((prev, run_start, count));
            }
        }
    }
}
