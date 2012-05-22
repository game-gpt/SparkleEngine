//! 3D 纹理网格 GPU 辅助：上传 RGBA 纹理并绘制 `TexMeshCmd`。
//!
//! 从 `game3d` 抽出，避免主文件继续膨胀。

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use spark_core::SparkError;
use spark_renderer::{DrawList3d, MeshResidentKey, RgbaImage, TexMeshVertex, TextureId};
use spark_shader::{create_builtin, BuiltinShader};
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
    _texture: wgpu::Texture,
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
    uniform: wgpu::Buffer,
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
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tex-mesh3d-uniform"),
            size: std::mem::size_of::<Uniforms3d>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tex-mesh3d-pl"),
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
            layout: Some(&pl),
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
            sampler,
            transient_vbo,
            transient_cap,
            textures: HashMap::new(),
            mesh_cache: HashMap::new(),
        }
    }

    pub fn ingest_uploads(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uploads: &[(TextureId, RgbaImage)],
    ) -> Result<(), SparkError> {
        for (id, img) in uploads {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("mesh3d-albedo"),
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
                label: Some("tex-mesh3d-bg"),
                layout: &self.bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform.as_entire_binding(),
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
                GpuTexture {
                    _texture: texture,
                    bind,
                },
            );
        }
        Ok(())
    }

    fn to_gpu(verts: &[TexMeshVertex]) -> Vec<TexVertGpu> {
        verts
            .iter()
            .map(|v| TexVertGpu {
                pos: v.pos,
                normal: v.normal,
                uv: v.uv,
                color: v.color,
            })
            .collect()
    }

    pub fn prepare_residents(&mut self, device: &wgpu::Device, list: &DrawList3d) {
        for mesh in list
            .tex_meshes
            .iter()
            .chain(list.tex_meshes_xlu.iter())
            .chain(list.tex_meshes_emissive.iter())
        {
            if let Some(key) = mesh.resident {
                self.ensure_resident(device, key, &mesh.vertices);
            }
        }
    }

    fn ensure_resident(
        &mut self,
        device: &wgpu::Device,
        key: MeshResidentKey,
        vertices: &[TexMeshVertex],
    ) {
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
        self.mesh_cache.insert(
            key.id.0,
            ResidentTexMesh {
                buffer,
                vertex_count: gpu.len() as u32,
                revision: key.revision,
            },
        );
    }

    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        queue: &wgpu::Queue,
        list: &DrawList3d,
        lights_bind: &'a wgpu::BindGroup,
    ) -> Result<(), SparkError> {
        self.draw_cmds(
            pass,
            queue,
            &list.tex_meshes,
            &list.view_proj,
            lights_bind,
            TexDrawKind::Opaque,
        )
    }

    /// Transparent pass：在不透明与 HUD 之间调用。
    pub fn draw_xlu<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        queue: &wgpu::Queue,
        list: &DrawList3d,
        lights_bind: &'a wgpu::BindGroup,
    ) -> Result<(), SparkError> {
        self.draw_cmds(
            pass,
            queue,
            &list.tex_meshes_xlu,
            &list.view_proj,
            lights_bind,
            TexDrawKind::Xlu,
        )
    }

    /// Emissive pass：透明之后、HUD 之前。
    pub fn draw_emissive<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        queue: &wgpu::Queue,
        list: &DrawList3d,
        lights_bind: &'a wgpu::BindGroup,
    ) -> Result<(), SparkError> {
        self.draw_cmds(
            pass,
            queue,
            &list.tex_meshes_emissive,
            &list.view_proj,
            lights_bind,
            TexDrawKind::Emissive,
        )
    }

    fn draw_cmds<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        queue: &wgpu::Queue,
        meshes: &[spark_renderer::TexMeshCmd],
        view_proj: &spark_geometry::Mat4,
        lights_bind: &'a wgpu::BindGroup,
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
        let vp = mat4_to_cols_pub(view_proj);
        for mesh in meshes {
            let Some(tex) = self.textures.get(&mesh.texture.0) else {
                continue;
            };
            let uniforms = Uniforms3d {
                view_proj: vp,
                model: mat4_to_cols_pub(&mesh.model),
            };
            queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniforms));
            pass.set_bind_group(0, &tex.bind, &[]);

            if let Some(key) = mesh.resident {
                let Some(entry) = self.mesh_cache.get(&key.id.0) else {
                    continue;
                };
                if entry.vertex_count == 0 {
                    continue;
                }
                let n = entry.vertex_count;
                pass.set_vertex_buffer(0, entry.buffer.slice(..));
                pass.draw(0..n, 0..1);
            } else {
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
