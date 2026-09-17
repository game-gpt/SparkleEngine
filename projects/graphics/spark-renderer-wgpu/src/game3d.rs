//! 3D 游戏宿主：透视网格 + 深度 + 可选 HUD + 指针锁定。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use spark_core::{Color, SparkError};
use spark_font::GlyphCache;
use spark_renderer::{
    DrawList, DrawList3d, FrameCtx, FrameLights3d, GameHost3d, Input, MeshCmd, MeshResidentKey,
    MeshVertex, WindowConfig,
};
use spark_shader::{BuiltinShader, create_builtin};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{CursorGrabMode, Window, WindowAttributes, WindowId};

use crate::winit_map;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms3d {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FrameLightsGpu {
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    ambient: [f32; 4],
    fog_color_density: [f32; 4],
    eye: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MeshVertGpu {
    pos: [f32; 3],
    normal: [f32; 3],
    color: [f32; 4],
}

impl FrameLightsGpu {
    fn from_lights(l: &FrameLights3d) -> Self {
        let d = l.sun_dir.normalized();
        Self {
            sun_dir: [d.x, d.y, d.z, 0.0],
            sun_color: l.sun_color.to_array(),
            ambient: l.ambient.to_array(),
            fog_color_density: [l.fog_color.r, l.fog_color.g, l.fog_color.b, l.fog_density],
            eye: [l.eye.x, l.eye.y, l.eye.z, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct HudUniforms {
    screen: [f32; 2],
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SolidVertex {
    pos: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GlyphVertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

fn mat4_to_cols(m: &spark_geometry::Mat4) -> [[f32; 4]; 4] {
    let c = m.as_cols();
    [
        [c[0], c[1], c[2], c[3]],
        [c[4], c[5], c[6], c[7]],
        [c[8], c[9], c[10], c[11]],
        [c[12], c[13], c[14], c[15]],
    ]
}

/// 供 `tex_mesh` 模块复用。
pub(crate) fn mat4_to_cols_pub(m: &spark_geometry::Mat4) -> [[f32; 4]; 4] {
    mat4_to_cols(m)
}

struct ResidentMesh {
    buffer: wgpu::Buffer,
    vertex_count: u32,
    revision: u32,
}

struct GpuState3d {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth_view: wgpu::TextureView,
    depth_tex: wgpu::Texture,
    /// 不透明网格：写深度，Less，前向光照。
    mesh_pipeline: wgpu::RenderPipeline,
    /// 天空 / 天体：不写深度，Always，无光照。
    sky_pipeline: wgpu::RenderPipeline,
    /// 天空加性发光：不写深度，Always，additive。
    sky_emissive_pipeline: wgpu::RenderPipeline,
    solid_pipeline: wgpu::RenderPipeline,
    glyph_pipeline: wgpu::RenderPipeline,
    mesh_bind: wgpu::BindGroup,
    mesh_uniform: wgpu::Buffer,
    lights_bind: wgpu::BindGroup,
    lights_uniform: wgpu::Buffer,
    solid_bind: wgpu::BindGroup,
    glyph_bind: wgpu::BindGroup,
    hud_uniform: wgpu::Buffer,
    glyph_tex: wgpu::Texture,
    glyph_view: wgpu::TextureView,
    glyph_cache: GlyphCache,
    /// 瞬时网格上传缓冲。
    mesh_vbo: wgpu::Buffer,
    solid_vbo: wgpu::Buffer,
    glyph_vbo: wgpu::Buffer,
    mesh_cap: u64,
    solid_cap: u64,
    glyph_cap: u64,
    /// 按 `MeshId` 驻留的 GPU 网格。
    mesh_cache: HashMap<u64, ResidentMesh>,
    tex_mesh: crate::tex_mesh::TexMeshGpu,
    skinned_mesh: crate::skinned_mesh::SkinnedMeshGpu,
}

impl GpuState3d {
    async fn new(window: Arc<Window>) -> Result<Self, SparkError> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::PRIMARY;
        let instance = wgpu::Instance::new(instance_desc);
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| SparkError::Message(format!("create surface: {e}")))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|e| SparkError::Message(format!("no adapter: {e}")))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("spark-3d"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                experimental_features: Default::default(),
                trace: Default::default(),
            })
            .await
            .map_err(|e| SparkError::Message(format!("request_device: {e}")))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let (depth_tex, depth_view) = make_depth(&device, width, height);

        let mesh_shader = create_builtin(&device, BuiltinShader::SolidMesh3d);
        let lit_mesh_shader = create_builtin(&device, BuiltinShader::LitSolidMesh3d);
        let solid_shader = create_builtin(&device, BuiltinShader::SolidQuad);
        let glyph_shader = create_builtin(&device, BuiltinShader::TexturedGlyph);

        let mesh_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mesh3d-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let lights_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame-lights-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let mesh_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh3d-uniform"),
            size: std::mem::size_of::<Uniforms3d>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let lights_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame-lights-uniform"),
            size: std::mem::size_of::<FrameLightsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mesh_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mesh3d-bg"),
            layout: &mesh_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: mesh_uniform.as_entire_binding(),
            }],
        });
        let lights_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame-lights-bg"),
            layout: &lights_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lights_uniform.as_entire_binding(),
            }],
        });
        let mesh_vert_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertGpu>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3,
                1 => Float32x3,
                2 => Float32x4
            ],
        };
        // SkyPass：无光照，仅 object uniform。
        let sky_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh3d-sky-pl"),
            bind_group_layouts: &[Some(&mesh_bgl)],
            immediate_size: 0,
        });
        // 不透明：object + frame lights。
        let mesh_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh3d-lit-pl"),
            bind_group_layouts: &[Some(&mesh_bgl), Some(&lights_bgl)],
            immediate_size: 0,
        });
        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh3d-lit"),
            layout: Some(&mesh_pl),
            vertex: wgpu::VertexState {
                module: &lit_mesh_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(mesh_vert_layout.clone())],
            },
            fragment: Some(wgpu::FragmentState {
                module: &lit_mesh_shader,
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
        // SkyPass：同顶点布局，关闭深度写入，Always 比较。
        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh3d-sky"),
            layout: Some(&sky_pl),
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(mesh_vert_layout)],
            },
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
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
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let sky_emissive_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh3d-sky-emissive"),
            layout: Some(&sky_pl),
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertGpu>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x3,
                        2 => Float32x4
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
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
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let mut glyph_cache = GlyphCache::load_system()?;
        let (aw, ah) = glyph_cache.atlas_size();
        let glyph_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph-atlas-3d"),
            size: wgpu::Extent3d {
                width: aw,
                height: ah,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let glyph_view = glyph_tex.create_view(&Default::default());
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &glyph_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            glyph_cache.atlas_bytes(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(aw),
                rows_per_image: Some(ah),
            },
            wgpu::Extent3d {
                width: aw,
                height: ah,
                depth_or_array_layers: 1,
            },
        );
        glyph_cache.take_dirty();

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let hud_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hud-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let glyph_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glyph-bgl-3d"),
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
        let hud_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hud-uniform"),
            size: std::mem::size_of::<HudUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let solid_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("solid-hud-bg"),
            layout: &hud_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: hud_uniform.as_entire_binding(),
            }],
        });
        let glyph_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph-hud-bg"),
            layout: &glyph_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: hud_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&glyph_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let solid_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("solid-hud-pl"),
            bind_group_layouts: &[Some(&hud_bgl)],
            immediate_size: 0,
        });
        let glyph_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("glyph-hud-pl"),
            bind_group_layouts: &[Some(&glyph_bgl)],
            immediate_size: 0,
        });
        let solid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("solid-hud"),
            layout: Some(&solid_pl),
            vertex: wgpu::VertexState {
                module: &solid_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<SolidVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &solid_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let glyph_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("glyph-hud"),
            layout: Some(&glyph_pl),
            vertex: wgpu::VertexState {
                module: &glyph_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GlyphVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &glyph_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let mesh_cap = 256_000u64;
        let solid_cap = 16_384u64;
        let glyph_cap = 32_768u64;
        let mesh_vbo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh-vbo"),
            size: mesh_cap * std::mem::size_of::<MeshVertGpu>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let solid_vbo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("solid-hud-vbo"),
            size: solid_cap * std::mem::size_of::<SolidVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let glyph_vbo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glyph-hud-vbo"),
            size: glyph_cap * std::mem::size_of::<GlyphVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        tracing::info!("GPU 3D ready");
        let tex_mesh = crate::tex_mesh::TexMeshGpu::new(&device, format, &lights_bgl);
        let skinned_mesh = crate::skinned_mesh::SkinnedMeshGpu::new(&device, format, &lights_bgl);
        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            depth_view,
            depth_tex,
            mesh_pipeline,
            sky_pipeline,
            sky_emissive_pipeline,
            solid_pipeline,
            glyph_pipeline,
            mesh_bind,
            mesh_uniform,
            lights_bind,
            lights_uniform,
            solid_bind,
            glyph_bind,
            hud_uniform,
            glyph_tex,
            glyph_view,
            glyph_cache,
            mesh_vbo,
            solid_vbo,
            glyph_vbo,
            mesh_cap,
            solid_cap,
            glyph_cap,
            mesh_cache: HashMap::new(),
            tex_mesh,
            skinned_mesh,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        let (tex, view) = make_depth(&self.device, width, height);
        self.depth_tex = tex;
        self.depth_view = view;
    }

    fn upload_mesh_verts(&self, vertices: &[MeshVertex]) -> Vec<MeshVertGpu> {
        vertices
            .iter()
            .map(|v| MeshVertGpu {
                pos: v.pos,
                normal: v.normal,
                color: v.color,
            })
            .collect()
    }

    /// 提交一组顶点色网格（天空或不透明）；`view_proj` 由调用方选择主相机或天空 VP。
    fn draw_mesh_cmds(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        meshes: &[MeshCmd],
        view_proj: &spark_geometry::Mat4,
    ) {
        let vp = mat4_to_cols(view_proj);
        for mesh in meshes {
            if mesh.vertices.is_empty() {
                continue;
            }
            let uniforms = Uniforms3d {
                view_proj: vp,
                model: mat4_to_cols(&mesh.model),
            };
            self.queue
                .write_buffer(&self.mesh_uniform, 0, bytemuck::bytes_of(&uniforms));

            if let Some(key) = mesh.resident {
                let Some(entry) = self.mesh_cache.get(&key.id.0) else {
                    continue;
                };
                if entry.vertex_count == 0 {
                    continue;
                }
                let vcount = entry.vertex_count;
                pass.set_vertex_buffer(0, entry.buffer.slice(..));
                pass.draw(0..vcount, 0..1);
            } else {
                let gpu_verts = self.upload_mesh_verts(&mesh.vertices);
                if gpu_verts.len() as u64 > self.mesh_cap {
                    continue;
                }
                self.queue
                    .write_buffer(&self.mesh_vbo, 0, bytemuck::cast_slice(&gpu_verts));
                pass.set_vertex_buffer(0, self.mesh_vbo.slice(..));
                pass.draw(0..gpu_verts.len() as u32, 0..1);
            }
        }
    }

    /// 确保驻留网格与 `revision` 一致，过期则重建 VBO。
    fn ensure_resident(&mut self, key: MeshResidentKey, vertices: &[MeshVertex]) {
        if let Some(entry) = self.mesh_cache.get(&key.id.0) {
            if entry.revision == key.revision && entry.vertex_count as usize == vertices.len() {
                return;
            }
        }
        let gpu_verts = self.upload_mesh_verts(vertices);
        let bytes = (gpu_verts.len() * std::mem::size_of::<MeshVertGpu>()) as u64;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("resident-mesh-vbo"),
            size: bytes.max(4),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !gpu_verts.is_empty() {
            self.queue
                .write_buffer(&buffer, 0, bytemuck::cast_slice(&gpu_verts));
        }
        self.mesh_cache.insert(
            key.id.0,
            ResidentMesh {
                buffer,
                vertex_count: gpu_verts.len() as u32,
                revision: key.revision,
            },
        );
    }

    fn render(&mut self, list: &DrawList3d) -> Result<(), SparkError> {
        let sw = self.config.width as f32;
        let sh = self.config.height as f32;

        // HUD verts
        let mut solids = Vec::new();
        for q in &list.hud.quads {
            let x0 = q.rect.x;
            let y0 = q.rect.y;
            let x1 = q.rect.x + q.rect.w;
            let y1 = q.rect.y + q.rect.h;
            let c = q.color.to_array();
            solids.extend_from_slice(&[
                SolidVertex {
                    pos: [x0, y0],
                    color: c,
                },
                SolidVertex {
                    pos: [x1, y0],
                    color: c,
                },
                SolidVertex {
                    pos: [x1, y1],
                    color: c,
                },
                SolidVertex {
                    pos: [x0, y0],
                    color: c,
                },
                SolidVertex {
                    pos: [x1, y1],
                    color: c,
                },
                SolidVertex {
                    pos: [x0, y1],
                    color: c,
                },
            ]);
        }
        let mut glyphs = Vec::new();
        for t in &list.hud.texts {
            let mut pen_x = t.pos.x;
            let baseline = t.pos.y + t.size;
            for ch in t.text.chars() {
                if ch == '\n' {
                    continue;
                }
                let Some(g) = self.glyph_cache.glyph(ch, t.size) else {
                    continue;
                };
                let x0 = pen_x;
                let y0 = baseline - g.bearing_y - g.height;
                let x1 = x0 + g.width;
                let y1 = y0 + g.height;
                let c = t.color.to_array();
                let (u0, v0, u1, v1) = (g.uv_min[0], g.uv_min[1], g.uv_max[0], g.uv_max[1]);
                glyphs.extend_from_slice(&[
                    GlyphVertex {
                        pos: [x0, y0],
                        uv: [u0, v0],
                        color: c,
                    },
                    GlyphVertex {
                        pos: [x1, y0],
                        uv: [u1, v0],
                        color: c,
                    },
                    GlyphVertex {
                        pos: [x1, y1],
                        uv: [u1, v1],
                        color: c,
                    },
                    GlyphVertex {
                        pos: [x0, y0],
                        uv: [u0, v0],
                        color: c,
                    },
                    GlyphVertex {
                        pos: [x1, y1],
                        uv: [u1, v1],
                        color: c,
                    },
                    GlyphVertex {
                        pos: [x0, y1],
                        uv: [u0, v1],
                        color: c,
                    },
                ]);
                pen_x += g.advance;
            }
        }
        if self.glyph_cache.take_dirty() {
            let (aw, ah) = self.glyph_cache.atlas_size();
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.glyph_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                self.glyph_cache.atlas_bytes(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(aw),
                    rows_per_image: Some(ah),
                },
                wgpu::Extent3d {
                    width: aw,
                    height: ah,
                    depth_or_array_layers: 1,
                },
            );
        }

        self.queue.write_buffer(
            &self.hud_uniform,
            0,
            bytemuck::bytes_of(&HudUniforms {
                screen: [sw, sh],
                _pad: [0.0; 2],
            }),
        );
        if solids.len() as u64 > self.solid_cap {
            return Err(SparkError::Message("HUD solid overflow".into()));
        }
        if glyphs.len() as u64 > self.glyph_cap {
            return Err(SparkError::Message("HUD glyph overflow".into()));
        }
        if !solids.is_empty() {
            self.queue
                .write_buffer(&self.solid_vbo, 0, bytemuck::cast_slice(&solids));
        }
        if !glyphs.is_empty() {
            self.queue
                .write_buffer(&self.glyph_vbo, 0, bytemuck::cast_slice(&glyphs));
        }

        let (frame, _) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex) => (tex, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(tex) => (tex, true),
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            _ => return Ok(()),
        };
        let view = frame.texture.create_view(&Default::default());

        // 渲染通道开始前完成驻留上传，避免与 pass 借用冲突。
        for mesh in list
            .sky_meshes
            .iter()
            .chain(list.sky_emissive_meshes.iter())
            .chain(list.meshes.iter())
        {
            if let Some(key) = mesh.resident {
                self.ensure_resident(key, &mesh.vertices);
            }
        }
        self.tex_mesh
            .ingest_uploads(&self.device, &self.queue, &list.texture_uploads)?;
        self.tex_mesh.prepare_residents(&self.device, list);
        self.skinned_mesh.prepare_residents(&self.device, list);

        let lights_gpu = FrameLightsGpu::from_lights(&list.lights);
        self.queue
            .write_buffer(&self.lights_uniform, 0, bytemuck::bytes_of(&lights_gpu));

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame3d"),
            });

        // SkyPass：先画天空/天体，再加性光晕；不写深度；随后清深度再画不透明世界。
        if !list.sky_meshes.is_empty() || !list.sky_emissive_meshes.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sky"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: list.clear.r as f64,
                            g: list.clear.g as f64,
                            b: list.clear.b as f64,
                            a: list.clear.a as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            if !list.sky_meshes.is_empty() {
                pass.set_pipeline(&self.sky_pipeline);
                pass.set_bind_group(0, &self.mesh_bind, &[]);
                self.draw_mesh_cmds(&mut pass, &list.sky_meshes, &list.sky_view_proj);
            }
            if !list.sky_emissive_meshes.is_empty() {
                pass.set_pipeline(&self.sky_emissive_pipeline);
                pass.set_bind_group(0, &self.mesh_bind, &[]);
                self.draw_mesh_cmds(&mut pass, &list.sky_emissive_meshes, &list.sky_view_proj);
            }
        }

        {
            let color_load = if list.sky_meshes.is_empty() && list.sky_emissive_meshes.is_empty() {
                wgpu::LoadOp::Clear(wgpu::Color {
                    r: list.clear.r as f64,
                    g: list.clear.g as f64,
                    b: list.clear.b as f64,
                    a: list.clear.a as f64,
                })
            } else {
                wgpu::LoadOp::Load
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d-opaque"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: color_load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        // 天空之后重新开始场景深度。
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.mesh_pipeline);
            pass.set_bind_group(0, &self.mesh_bind, &[]);
            pass.set_bind_group(1, &self.lights_bind, &[]);
            self.draw_mesh_cmds(&mut pass, &list.meshes, &list.view_proj);
            self.tex_mesh
                .draw(&mut pass, &self.queue, list, &self.lights_bind)?;
            self.skinned_mesh.draw(
                &mut pass,
                &self.device,
                &self.queue,
                list,
                &self.lights_bind,
                &list.view_proj,
            )?;
        }

        if !list.tex_meshes_xlu.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d-transparent"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            self.tex_mesh
                .draw_xlu(&mut pass, &self.queue, list, &self.lights_bind)?;
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("hud"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            if !solids.is_empty() {
                pass.set_pipeline(&self.solid_pipeline);
                pass.set_bind_group(0, &self.solid_bind, &[]);
                pass.set_vertex_buffer(0, self.solid_vbo.slice(..));
                pass.draw(0..solids.len() as u32, 0..1);
            }
            if !glyphs.is_empty() {
                pass.set_pipeline(&self.glyph_pipeline);
                pass.set_bind_group(0, &self.glyph_bind, &[]);
                pass.set_vertex_buffer(0, self.glyph_vbo.slice(..));
                pass.draw(0..glyphs.len() as u32, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        self.window.pre_present_notify();
        self.queue.present(frame);
        let _ = (&self.glyph_view, Color::rgb(0.0, 0.0, 0.0));
        Ok(())
    }
}

fn make_depth(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::Texture, wgpu::TextureView) {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = tex.create_view(&Default::default());
    (tex, view)
}

struct HostApp3d<H: GameHost3d> {
    config: WindowConfig,
    host: H,
    input: Input,
    state: Option<GpuState3d>,
    last: Instant,
    scale: f32,
    grab_applied: bool,
    modifiers: ModifiersState,
}

impl<H: GameHost3d> ApplicationHandler for HostApp3d<H> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::error!(?e, "create_window");
                event_loop.exit();
                return;
            }
        };
        self.scale = window.scale_factor() as f32;
        match pollster::block_on(GpuState3d::new(window)) {
            Ok(s) => self.state = Some(s),
            Err(e) => {
                tracing::error!(?e, "gpu3d init");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(s) = self.state.as_mut() {
                    s.resize(size.width, size.height);
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = scale_factor as f32;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(k) = winit_map::key(event.physical_key) {
                    self.input
                        .on_key(k, winit_map::button_state(event.state));
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(b) = winit_map::mouse_btn(button) {
                    self.input
                        .on_mouse_button(b, winit_map::button_state(state));
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.on_cursor(
                    position.x as f32 / self.scale,
                    position.y as f32 / self.scale,
                );
            }
            WindowEvent::ModifiersChanged(m) => {
                self.modifiers = m.state();
            }
            WindowEvent::RedrawRequested => self.frame(event_loop),
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.input
                .on_mouse_delta(delta.0 as f32, delta.1 as f32);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(s) = self.state.as_ref() {
            s.window.request_redraw();
        }
    }
}

impl<H: GameHost3d> HostApp3d<H> {
    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let Some(gpu) = self.state.as_mut() else {
            return;
        };

        let want_grab = self.host.cursor_grab();
        if want_grab != self.grab_applied {
            if want_grab {
                let _ = gpu.window.set_cursor_grab(CursorGrabMode::Locked)
                    .or_else(|_| gpu.window.set_cursor_grab(CursorGrabMode::Confined));
                gpu.window.set_cursor_visible(false);
            } else {
                let _ = gpu.window.set_cursor_grab(CursorGrabMode::None);
                gpu.window.set_cursor_visible(true);
            }
            self.grab_applied = want_grab;
        }

        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().clamp(1.0 / 240.0, 0.05);
        self.last = now;
        let sw = gpu.config.width as f32;
        let sh = gpu.config.height as f32;

        {
            let frame = FrameCtx {
                input: &self.input,
                dt,
                screen_w: sw,
                screen_h: sh,
            };
            self.host.update(&frame);
        }
        self.input.begin_frame();

        if self.host.should_exit() {
            event_loop.exit();
            return;
        }

        let clear = Color::rgba(
            self.config.clear_color[0] as f32,
            self.config.clear_color[1] as f32,
            self.config.clear_color[2] as f32,
            self.config.clear_color[3] as f32,
        );
        let mut draw = DrawList3d::new(clear, spark_geometry::Mat4::IDENTITY);
        self.host.draw(&mut draw);
        if let Err(e) = gpu.render(&draw) {
            tracing::error!(?e, "render3d failed");
            event_loop.exit();
            return;
        }
        gpu.window.request_redraw();
        let _ = DrawList::new(clear);
        let _ = self.modifiers;
    }
}

/// 窗口事件泵 + GPU 提交（3D）。帧相位编排请走 `spark_engine::run_game_3d`。
pub fn run_window_3d<H: GameHost3d + 'static>(
    config: WindowConfig,
    host: H,
) -> Result<(), SparkError> {
    let event_loop = EventLoop::new().map_err(|e| SparkError::Message(e.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = HostApp3d {
        config,
        host,
        input: Input::default(),
        state: None,
        last: Instant::now(),
        scale: 1.0,
        grab_applied: false,
        modifiers: ModifiersState::default(),
    };
    event_loop
        .run_app(&mut app)
        .map_err(|e| SparkError::Message(e.to_string()))
}
