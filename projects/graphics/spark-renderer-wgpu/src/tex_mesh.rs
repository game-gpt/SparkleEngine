//! 3D 纹理网格 GPU 辅助：上传 RGBA 纹理并绘制 `TexMeshCmd`。
//!
//! 从 `game3d` 抽出，避免主文件继续膨胀。

use std::{cell::Cell, collections::HashMap};

use bytemuck::{Pod, Zeroable};
use spark_renderer::{DrawList3d, MeshResidentKey, TexMeshVertex, TextureId, TextureUpload};
use spark_shader::{BuiltinShader, create_builtin};
use spark_types::SparkError;
use wgpu::util::DeviceExt;

use crate::game3d::mat4_to_cols_pub;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TexVertGpu {
    pos: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms3d {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

struct GpuTexture {
    texture: wgpu::Texture,
    bind: wgpu::BindGroup,
}

struct ResidentTexMesh {
    buffer: wgpu::Buffer,
    vertex_count: u32,
    revision: u32,
}

/// 纹理网格管线与缓存。
pub struct TexMeshGpu {
    pipeline: wgpu::RenderPipeline,
    /// Transparent：测深、不写深、双面（树叶/玻璃）。
    pipeline_xlu: wgpu::RenderPipeline,
    /// Emissive：测深、不写深、additive（岩浆/引擎）。
    pipeline_emissive: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    /// 动态 object uniform 环（每 draw 一槽，submit 前一次性写入）。
    uniform: wgpu::Buffer,
    uniform_stride: u64,
    uniform_slots: usize,
    /// 本帧环游标（opaque / xlu / emissive 累加）。
    uniform_next: Cell<usize>,
    sampler: wgpu::Sampler,
    transient_vbo: wgpu::Buffer,
    transient_cap: u64,
    textures: HashMap<u32, GpuTexture>,
    mesh_cache: HashMap<u64, ResidentTexMesh>,
}

impl TexMeshGpu {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        lights_bgl: &wgpu::BindGroupLayout,
        shadow_bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = create_builtin(device, BuiltinShader::TexturedMesh3d);
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tex-mesh3d-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: crate::dyn_ubo::binding_size(std::mem::size_of::<Uniforms3d>() as u64),
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
        let uniform_stride = crate::dyn_ubo::uniform_stride(device, std::mem::size_of::<Uniforms3d>() as u64);
        let uniform_slots = 512usize;
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tex-mesh3d-uniform-ring"),
            size: uniform_stride * uniform_slots as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tex-mesh3d-pl"),
            bind_group_layouts: &[Some(&bgl), Some(lights_bgl), Some(shadow_bgl)],
            immediate_size: 0,
        });
        let pl_emissive = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tex-mesh3d-emissive-pl"),
            bind_group_layouts: &[Some(&bgl), Some(lights_bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tex-mesh3d"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TexVertGpu>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x3,
                        2 => Float32x2,
                        3 => Float32x4
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let pipeline_xlu = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tex-mesh3d-xlu"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TexVertGpu>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x3,
                        2 => Float32x2,
                        3 => Float32x4
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                // 树叶/玻璃双面可见。
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let emissive_shader = create_builtin(device, BuiltinShader::EmissiveMesh3d);
        let pipeline_emissive = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tex-mesh3d-emissive"),
            layout: Some(&pl_emissive),
            vertex: wgpu::VertexState {
                module: &emissive_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TexVertGpu>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x3,
                        2 => Float32x2,
                        3 => Float32x4
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &emissive_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let transient_cap = 256_000u64;
        let transient_vbo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tex-mesh-transient-vbo"),
            size: transient_cap * std::mem::size_of::<TexVertGpu>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            pipeline_xlu,
            pipeline_emissive,
            bgl,
            uniform,
            uniform_stride,
            uniform_slots,
            uniform_next: Cell::new(0),
            sampler,
            transient_vbo,
            transient_cap,
            textures: HashMap::new(),
            mesh_cache: HashMap::new(),
        }
    }

    /// 每帧渲染开始时重置 object uniform 环游标。
    pub fn begin_frame(&self) {
        self.uniform_next.set(0);
    }

    fn object_binding(&self) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: &self.uniform,
            offset: 0,
            size: crate::dyn_ubo::binding_size(std::mem::size_of::<Uniforms3d>() as u64),
        })
    }

    fn make_tex_bind(&self, device: &wgpu::Device, view: &wgpu::TextureView) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tex-mesh3d-bg"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.object_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        })
    }

    fn ensure_uniform_slots(&mut self, device: &wgpu::Device, need: usize) {
        if need <= self.uniform_slots {
            return;
        }
        let slots = need.next_power_of_two().max(self.uniform_slots * 2);
        self.uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tex-mesh3d-uniform-ring"),
            size: self.uniform_stride * slots as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.uniform_slots = slots;
        // 环缓冲换新后重建所有纹理 bind（引用旧 buffer 无效）。
        let keys: Vec<u32> = self.textures.keys().copied().collect();
        for id in keys {
            let Some(tex) = self.textures.remove(&id)
            else {
                continue;
            };
            let view = tex.texture.create_view(&Default::default());
            let bind = self.make_tex_bind(device, &view);
            self.textures.insert(id, GpuTexture { texture: tex.texture, bind });
        }
    }

    pub fn ingest_uploads(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uploads: &[(TextureId, TextureUpload)],
    ) -> Result<(), SparkError> {
        for (id, upload) in uploads {
            let texture = crate::texture_upload::create_texture_from_upload(device, queue, upload)?;
            let view = texture.create_view(&Default::default());
            let bind = self.make_tex_bind(device, &view);
            self.textures.insert(id.0, GpuTexture { texture, bind });
        }
        Ok(())
    }

    fn to_gpu(verts: &[TexMeshVertex]) -> Vec<TexVertGpu> {
        verts.iter().map(|v| TexVertGpu { pos: v.pos, normal: v.normal, uv: v.uv, color: v.color }).collect()
    }

    pub fn prepare_residents(&mut self, device: &wgpu::Device, list: &DrawList3d) {
        let draw_n = list.tex_meshes.len() + list.tex_meshes_xlu.len() + list.tex_meshes_emissive.len();
        self.ensure_uniform_slots(device, draw_n);
        let mut uploads = 0usize;
        for mesh in list.tex_meshes.iter().chain(list.tex_meshes_xlu.iter()).chain(list.tex_meshes_emissive.iter()) {
            if let Some(key) = mesh.resident {
                if !Self::resident_stale(self.mesh_cache.get(&key.id.0), key, mesh.vertices.len()) {
                    continue;
                }
                if uploads >= crate::RESIDENT_UPLOADS_PER_FRAME {
                    continue;
                }
                self.ensure_resident(device, key, &mesh.vertices);
                uploads += 1;
            }
        }
    }

    fn resident_stale(entry: Option<&ResidentTexMesh>, key: MeshResidentKey, vertex_count: usize) -> bool {
        match entry {
            Some(e) => e.revision != key.revision || e.vertex_count as usize != vertex_count,
            None => vertex_count > 0,
        }
    }

    fn ensure_resident(&mut self, device: &wgpu::Device, key: MeshResidentKey, vertices: &[TexMeshVertex]) {
        if let Some(e) = self.mesh_cache.get(&key.id.0) {
            if e.revision == key.revision && e.vertex_count as usize == vertices.len() {
                return;
            }
        }
        let gpu = Self::to_gpu(vertices);
        if gpu.is_empty() {
            self.mesh_cache.remove(&key.id.0);
            return;
        }
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tex-mesh-resident"),
            contents: bytemuck::cast_slice(&gpu),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.mesh_cache.insert(key.id.0, ResidentTexMesh { buffer, vertex_count: gpu.len() as u32, revision: key.revision });
    }

    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        queue: &wgpu::Queue,
        list: &DrawList3d,
        lights_bind: &'a wgpu::BindGroup,
        shadow_bind: &'a wgpu::BindGroup,
    ) -> Result<(), SparkError> {
        self.draw_cmds(pass, queue, &list.tex_meshes, &list.view_proj, lights_bind, Some(shadow_bind), TexDrawKind::Opaque)
    }

    /// Transparent pass：在不透明与 HUD 之间调用。
    pub fn draw_xlu<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        queue: &wgpu::Queue,
        list: &DrawList3d,
        lights_bind: &'a wgpu::BindGroup,
        shadow_bind: &'a wgpu::BindGroup,
    ) -> Result<(), SparkError> {
        self.draw_cmds(pass, queue, &list.tex_meshes_xlu, &list.view_proj, lights_bind, Some(shadow_bind), TexDrawKind::Xlu)
    }

    /// Emissive pass：透明之后、HUD 之前。
    pub fn draw_emissive<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        queue: &wgpu::Queue,
        list: &DrawList3d,
        lights_bind: &'a wgpu::BindGroup,
    ) -> Result<(), SparkError> {
        self.draw_cmds(pass, queue, &list.tex_meshes_emissive, &list.view_proj, lights_bind, None, TexDrawKind::Emissive)
    }

    fn draw_cmds<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        queue: &wgpu::Queue,
        meshes: &[spark_renderer::TexMeshCmd],
        view_proj: &spark_geometry::Mat4,
        lights_bind: &'a wgpu::BindGroup,
        shadow_bind: Option<&'a wgpu::BindGroup>,
        kind: TexDrawKind,
    ) -> Result<(), SparkError> {
        if meshes.is_empty() {
            return Ok(());
        }
        pass.set_pipeline(match kind {
            TexDrawKind::Opaque => &self.pipeline,
            TexDrawKind::Xlu => &self.pipeline_xlu,
            TexDrawKind::Emissive => &self.pipeline_emissive,
        });
        pass.set_bind_group(1, lights_bind, &[]);
        if let Some(shadow) = shadow_bind {
            pass.set_bind_group(2, shadow, &[]);
        }
        let vp = mat4_to_cols_pub(view_proj);
        // 先为每条 draw 写入独立槽，再开画；游标跨 opaque/xlu/emissive 累加。
        let mut slot = self.uniform_next.get();
        let mut draw_slots: Vec<(u32, usize)> = Vec::with_capacity(meshes.len());
        for (mi, mesh) in meshes.iter().enumerate() {
            if !self.textures.contains_key(&mesh.texture.0) {
                continue;
            }
            let drawable = if let Some(key) = mesh.resident {
                self.mesh_cache.get(&key.id.0).map(|e| e.vertex_count > 0).unwrap_or(false)
            }
            else {
                !mesh.vertices.is_empty() && (mesh.vertices.len() as u64) <= self.transient_cap
            };
            if !drawable {
                continue;
            }
            if slot >= self.uniform_slots {
                break;
            }
            let uniforms = Uniforms3d { view_proj: vp, model: mat4_to_cols_pub(&mesh.model) };
            queue.write_buffer(&self.uniform, slot as u64 * self.uniform_stride, bytemuck::bytes_of(&uniforms));
            draw_slots.push(((slot as u64 * self.uniform_stride) as u32, mi));
            slot += 1;
        }
        self.uniform_next.set(slot);

        for (dyn_off, mi) in draw_slots {
            let mesh = &meshes[mi];
            let Some(tex) = self.textures.get(&mesh.texture.0)
            else {
                continue;
            };
            pass.set_bind_group(0, &tex.bind, &[dyn_off]);

            if let Some(key) = mesh.resident {
                let Some(entry) = self.mesh_cache.get(&key.id.0)
                else {
                    continue;
                };
                if entry.vertex_count == 0 {
                    continue;
                }
                let n = entry.vertex_count;
                pass.set_vertex_buffer(0, entry.buffer.slice(..));
                pass.draw(0..n, 0..1);
            }
            else {
                let gpu = Self::to_gpu(&mesh.vertices);
                if gpu.len() as u64 > self.transient_cap {
                    continue;
                }
                queue.write_buffer(&self.transient_vbo, 0, bytemuck::cast_slice(&gpu));
                pass.set_vertex_buffer(0, self.transient_vbo.slice(..));
                pass.draw(0..gpu.len() as u32, 0..1);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum TexDrawKind {
    Opaque,
    Xlu,
    Emissive,
}
