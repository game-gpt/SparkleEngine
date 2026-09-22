//! 3D 游戏宿主：透视网格 + 深度 + 可选 HUD + 指针锁定。

use std::{cell::Cell, collections::HashMap, sync::Arc, time::Instant};

use bytemuck::{Pod, Zeroable};
use spark_font::GlyphCache;
use spark_renderer::{DrawList, DrawList3d, FrameCtx, FrameLights3d, GameHost3d, Input, MeshCmd, MeshResidentKey, MeshVertex, WindowConfig};
use spark_shader::{BuiltinShader, create_builtin};
use spark_types::{Color, SparkError, codes};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{DeviceEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::ModifiersState,
    window::{CursorGrabMode, Window, WindowAttributes, WindowId},
};

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
            eye: [l.eye.x, l.eye.y, l.eye.z, l.exposure],
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
    [[c[0], c[1], c[2], c[3]], [c[4], c[5], c[6], c[7]], [c[8], c[9], c[10], c[11]], [c[12], c[13], c[14], c[15]]]
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
    /// 顶点色半透明：测深不写深。
    mesh_xlu_pipeline: wgpu::RenderPipeline,
    /// 顶点色自发光：测深不写深、additive。
    mesh_emissive_pipeline: wgpu::RenderPipeline,
    /// 天空 / 天体：不写深度，Always，无光照。
    sky_pipeline: wgpu::RenderPipeline,
    /// 大气穹顶：不写深度，Always，消费 FrameLights。
    sky_atmosphere_pipeline: wgpu::RenderPipeline,
    /// 天空加性发光：不写深度，Always，additive。
    sky_emissive_pipeline: wgpu::RenderPipeline,
    solid_pipeline: wgpu::RenderPipeline,
    glyph_pipeline: wgpu::RenderPipeline,
    mesh_bind: wgpu::BindGroup,
    mesh_uniform: wgpu::Buffer,
    mesh_uniform_stride: u64,
    mesh_uniform_slots: usize,
    /// 本帧 object uniform 环游标（跨天空/不透明/透明/VM 累加，避免同偏移覆盖）。
    mesh_uniform_next: Cell<usize>,
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
    bloom: crate::bloom::BloomGpu,
    shadow: crate::shadow::ShadowMapGpu,
}

impl GpuState3d {
    async fn new(window: Arc<Window>) -> Result<Self, SparkError> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::PRIMARY;
        let instance = wgpu::Instance::new(instance_desc);
        let surface = instance.create_surface(window.clone()).map_err(|e| SparkError::new(codes::gpu_surface()).caused_by(e))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|_| {
                SparkError::new(codes::gpu_adapter()).arg("reason", spark_types::ErrorArg::String(std::sync::Arc::from("no_adapter")))
            })?;
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
            .map_err(|e| SparkError::new(codes::gpu_device()).caused_by(e))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
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
        let sky_atm_shader = create_builtin(&device, BuiltinShader::SkyAtmosphere3d);
        let solid_shader = create_builtin(&device, BuiltinShader::SolidQuad);
        let glyph_shader = create_builtin(&device, BuiltinShader::TexturedGlyph);

        let mesh_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mesh3d-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: crate::dyn_ubo::binding_size(std::mem::size_of::<Uniforms3d>() as u64),
                },
                count: None,
            }],
        });
        let lights_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame-lights-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let mesh_uniform_stride = crate::dyn_ubo::uniform_stride(&device, std::mem::size_of::<Uniforms3d>() as u64);
        let mesh_uniform_slots = 512usize;
        let mesh_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh3d-uniform-ring"),
            size: mesh_uniform_stride * mesh_uniform_slots as u64,
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
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &mesh_uniform,
                    offset: 0,
                    size: crate::dyn_ubo::binding_size(std::mem::size_of::<Uniforms3d>() as u64),
                }),
            }],
        });
        let lights_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame-lights-bg"),
            layout: &lights_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: lights_uniform.as_entire_binding() }],
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
        let shadow = crate::shadow::ShadowMapGpu::new(&device);
        // 不透明 / 透明 lit：object + frame lights + sun shadow。
        let mesh_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh3d-lit-pl"),
            bind_group_layouts: &[Some(&mesh_bgl), Some(&lights_bgl), Some(shadow.sample_bgl())],
            immediate_size: 0,
        });
        // 大气穹顶：object + lights，无阴影采样。
        let mesh_pl_lights = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh3d-lights-pl"),
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
        let mesh_xlu_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh3d-lit-xlu"),
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
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let mesh_emissive_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh3d-emissive"),
            layout: Some(&sky_pl),
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(mesh_vert_layout.clone())],
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
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
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
        let sky_atmosphere_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh3d-sky-atmosphere"),
            layout: Some(&mesh_pl_lights),
            vertex: wgpu::VertexState {
                module: &sky_atm_shader,
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
                module: &sky_atm_shader,
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
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, cull_mode: None, ..Default::default() },
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
            size: wgpu::Extent3d { width: aw, height: ah, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let glyph_view = glyph_tex.create_view(&Default::default());
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &glyph_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            glyph_cache.atlas_bytes(),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(aw), rows_per_image: Some(ah) },
            wgpu::Extent3d { width: aw, height: ah, depth_or_array_layers: 1 },
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
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let glyph_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glyph-bgl-3d"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
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
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: hud_uniform.as_entire_binding() }],
        });
        let glyph_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph-hud-bg"),
            layout: &glyph_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: hud_uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&glyph_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
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
        // 全屏地图等 HUD 色块可达数万顶点；不足时由 `ensure_solid_cap` 扩容。
        let solid_cap = 65_536u64;
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

        tracing::info!(event = "spark.renderer.gpu3d_ready");
        let tex_mesh = crate::tex_mesh::TexMeshGpu::new(&device, format, &lights_bgl, shadow.sample_bgl());
        let skinned_mesh = crate::skinned_mesh::SkinnedMeshGpu::new(&device, format, &lights_bgl);
        let bloom = crate::bloom::BloomGpu::new(&device, format);
        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            depth_view,
            depth_tex,
            mesh_pipeline,
            mesh_xlu_pipeline,
            mesh_emissive_pipeline,
            sky_pipeline,
            sky_atmosphere_pipeline,
            sky_emissive_pipeline,
            solid_pipeline,
            glyph_pipeline,
            mesh_bind,
            mesh_uniform,
            mesh_uniform_stride,
            mesh_uniform_slots,
            mesh_uniform_next: Cell::new(0),
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
            bloom,
            shadow,
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
        self.bloom.resize(&self.device, width, height);
    }

    fn upload_mesh_verts(&self, vertices: &[MeshVertex]) -> Vec<MeshVertGpu> {
        vertices.iter().map(|v| MeshVertGpu { pos: v.pos, normal: v.normal, color: v.color }).collect()
    }

    /// 提交一组顶点色网格（天空或不透明）；`view_proj` 由调用方选择主相机或天空 VP。
    fn draw_mesh_cmds(&self, pass: &mut wgpu::RenderPass<'_>, meshes: &[MeshCmd], view_proj: &spark_geometry::Mat4) {
        let vp = mat4_to_cols(view_proj);
        // 每条 draw 独立槽；游标跨 pass 累加，因 `write_buffer` 在 submit 前合并。
        let mut slot = self.mesh_uniform_next.get();
        let mut planned: Vec<(u32, usize)> = Vec::with_capacity(meshes.len());
        for (mi, mesh) in meshes.iter().enumerate() {
            if mesh.vertices.is_empty() && mesh.resident.is_none() {
                continue;
            }
            let drawable = if let Some(key) = mesh.resident {
                self.mesh_cache.get(&key.id.0).map(|e| e.vertex_count > 0).unwrap_or(false)
            }
            else {
                !mesh.vertices.is_empty() && (mesh.vertices.len() as u64) <= self.mesh_cap
            };
            if !drawable {
                continue;
            }
            if slot >= self.mesh_uniform_slots {
                break;
            }
            let uniforms = Uniforms3d { view_proj: vp, model: mat4_to_cols(&mesh.model) };
            let off = slot as u64 * self.mesh_uniform_stride;
            self.queue.write_buffer(&self.mesh_uniform, off, bytemuck::bytes_of(&uniforms));
            planned.push((off as u32, mi));
            slot += 1;
        }
        self.mesh_uniform_next.set(slot);
        for (dyn_off, mi) in planned {
            let mesh = &meshes[mi];
            pass.set_bind_group(0, &self.mesh_bind, &[dyn_off]);
            if let Some(key) = mesh.resident {
                let Some(entry) = self.mesh_cache.get(&key.id.0)
                else {
                    continue;
                };
                if entry.vertex_count == 0 {
                    continue;
                }
                let vcount = entry.vertex_count;
                pass.set_vertex_buffer(0, entry.buffer.slice(..));
                pass.draw(0..vcount, 0..1);
            }
            else {
                let gpu_verts = self.upload_mesh_verts(&mesh.vertices);
                if gpu_verts.len() as u64 > self.mesh_cap {
                    continue;
                }
                self.queue.write_buffer(&self.mesh_vbo, 0, bytemuck::cast_slice(&gpu_verts));
                pass.set_vertex_buffer(0, self.mesh_vbo.slice(..));
                pass.draw(0..gpu_verts.len() as u32, 0..1);
            }
        }
    }

    /// 确保驻留网格与 `revision` 一致，过期则重建 VBO。
    fn resident_stale(entry: Option<&ResidentMesh>, key: MeshResidentKey, vertex_count: usize) -> bool {
        match entry {
            Some(e) => e.revision != key.revision || e.vertex_count as usize != vertex_count,
            None => vertex_count > 0,
        }
    }

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
            self.queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&gpu_verts));
        }
        self.mesh_cache.insert(key.id.0, ResidentMesh { buffer, vertex_count: gpu_verts.len() as u32, revision: key.revision });
    }

    fn ensure_solid_cap(&mut self, need: u64) -> Result<(), SparkError> {
        if need <= self.solid_cap {
            return Ok(());
        }
        let mut cap = self.solid_cap.max(4096);
        while cap < need {
            cap = cap.saturating_mul(2);
        }
        self.solid_vbo = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("solid-hud-vbo"),
            size: cap * std::mem::size_of::<SolidVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.solid_cap = cap;
        Ok(())
    }

    fn ensure_glyph_cap(&mut self, need: u64) -> Result<(), SparkError> {
        if need <= self.glyph_cap {
            return Ok(());
        }
        let mut cap = self.glyph_cap.max(4096);
        while cap < need {
            cap = cap.saturating_mul(2);
        }
        self.glyph_vbo = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glyph-hud-vbo"),
            size: cap * std::mem::size_of::<GlyphVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.glyph_cap = cap;
        Ok(())
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
                SolidVertex { pos: [x0, y0], color: c },
                SolidVertex { pos: [x1, y0], color: c },
                SolidVertex { pos: [x1, y1], color: c },
                SolidVertex { pos: [x0, y0], color: c },
                SolidVertex { pos: [x1, y1], color: c },
                SolidVertex { pos: [x0, y1], color: c },
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
                let Some(g) = self.glyph_cache.glyph(ch, t.size)
                else {
                    continue;
                };
                let x0 = pen_x;
                let y0 = baseline - g.bearing_y - g.height;
                let x1 = x0 + g.width;
                let y1 = y0 + g.height;
                let c = t.color.to_array();
                let (u0, v0, u1, v1) = (g.uv_min[0], g.uv_min[1], g.uv_max[0], g.uv_max[1]);
                glyphs.extend_from_slice(&[
                    GlyphVertex { pos: [x0, y0], uv: [u0, v0], color: c },
                    GlyphVertex { pos: [x1, y0], uv: [u1, v0], color: c },
                    GlyphVertex { pos: [x1, y1], uv: [u1, v1], color: c },
                    GlyphVertex { pos: [x0, y0], uv: [u0, v0], color: c },
                    GlyphVertex { pos: [x1, y1], uv: [u1, v1], color: c },
                    GlyphVertex { pos: [x0, y1], uv: [u0, v1], color: c },
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
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(aw), rows_per_image: Some(ah) },
                wgpu::Extent3d { width: aw, height: ah, depth_or_array_layers: 1 },
            );
        }

        self.queue.write_buffer(&self.hud_uniform, 0, bytemuck::bytes_of(&HudUniforms { screen: [sw, sh], _pad: [0.0; 2] }));
        if solids.len() as u64 > self.solid_cap {
            self.ensure_solid_cap(solids.len() as u64)?;
        }
        if glyphs.len() as u64 > self.glyph_cap {
            self.ensure_glyph_cap(glyphs.len() as u64)?;
        }
        if !solids.is_empty() {
            self.queue.write_buffer(&self.solid_vbo, 0, bytemuck::cast_slice(&solids));
        }
        if !glyphs.is_empty() {
            self.queue.write_buffer(&self.glyph_vbo, 0, bytemuck::cast_slice(&glyphs));
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

        // 每帧重置 object uniform 环游标（mesh / tex-mesh 各自独立）。
        self.mesh_uniform_next.set(0);
        self.tex_mesh.begin_frame();

        // 渲染通道开始前完成驻留上传，避免与 pass 借用冲突。
        // 每帧限量，避免 remesh 洪峰一次 create_buffer 过多。
        let mut uploads = 0usize;
        for mesh in list
            .sky_atmosphere_meshes
            .iter()
            .chain(list.sky_meshes.iter())
            .chain(list.sky_emissive_meshes.iter())
            .chain(list.meshes.iter())
            .chain(list.meshes_xlu.iter())
            .chain(list.meshes_emissive.iter())
            .chain(list.view_model_meshes.iter())
        {
            if let Some(key) = mesh.resident {
                if !Self::resident_stale(self.mesh_cache.get(&key.id.0), key, mesh.vertices.len()) {
                    continue;
                }
                if uploads >= crate::RESIDENT_UPLOADS_PER_FRAME {
                    continue;
                }
                self.ensure_resident(key, &mesh.vertices);
                uploads += 1;
            }
        }
        self.tex_mesh.ingest_uploads(&self.device, &self.queue, &list.texture_uploads)?;
        self.tex_mesh.prepare_residents(&self.device, list);
        self.skinned_mesh.prepare_residents(&self.device, list);

        let lights_gpu = FrameLightsGpu::from_lights(&list.lights);
        self.queue.write_buffer(&self.lights_uniform, 0, bytemuck::bytes_of(&lights_gpu));

        let use_bloom = list.bloom_strength > 0.001;
        if use_bloom {
            self.bloom.resize(&self.device, self.config.width, self.config.height);
        }
        // 克隆场景 RT 视图，避免与后续 `bloom.apply` 借用冲突。
        let scene_color = if use_bloom { self.bloom.scene_view().map(|v| v.clone()) } else { None };
        let color_view = scene_color.as_ref().unwrap_or(&view);

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame3d") });

        self.shadow.write_params(&self.queue, list);
        self.shadow.render_casters(&mut encoder, &self.device, &self.queue, list);

        // SkyPass：大气穹顶 → 顶点色天体 → 加性光晕；不写深度；随后清深度再画不透明世界。
        let has_sky = !list.sky_atmosphere_meshes.is_empty() || !list.sky_meshes.is_empty() || !list.sky_emissive_meshes.is_empty();
        if has_sky {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sky"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
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
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            if !list.sky_atmosphere_meshes.is_empty() {
                pass.set_pipeline(&self.sky_atmosphere_pipeline);
                pass.set_bind_group(1, &self.lights_bind, &[]);
                self.draw_mesh_cmds(&mut pass, &list.sky_atmosphere_meshes, &list.sky_view_proj);
            }
            if !list.sky_meshes.is_empty() {
                pass.set_pipeline(&self.sky_pipeline);
                self.draw_mesh_cmds(&mut pass, &list.sky_meshes, &list.sky_view_proj);
            }
            if !list.sky_emissive_meshes.is_empty() {
                pass.set_pipeline(&self.sky_emissive_pipeline);
                self.draw_mesh_cmds(&mut pass, &list.sky_emissive_meshes, &list.sky_view_proj);
            }
        }

        {
            let color_load = if !has_sky {
                wgpu::LoadOp::Clear(wgpu::Color {
                    r: list.clear.r as f64,
                    g: list.clear.g as f64,
                    b: list.clear.b as f64,
                    a: list.clear.a as f64,
                })
            }
            else {
                wgpu::LoadOp::Load
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d-opaque"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: color_load, store: wgpu::StoreOp::Store },
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
            pass.set_bind_group(1, &self.lights_bind, &[]);
            pass.set_bind_group(2, self.shadow.sample_bind(), &[]);
            self.draw_mesh_cmds(&mut pass, &list.meshes, &list.view_proj);
            self.tex_mesh.draw(&mut pass, &self.queue, list, &self.lights_bind, self.shadow.sample_bind())?;
            self.skinned_mesh.draw(&mut pass, &self.device, &self.queue, list, &self.lights_bind, &list.view_proj)?;
        }

        if !list.tex_meshes_xlu.is_empty() || !list.meshes_xlu.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d-transparent"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            if !list.meshes_xlu.is_empty() {
                pass.set_pipeline(&self.mesh_xlu_pipeline);
                pass.set_bind_group(1, &self.lights_bind, &[]);
                pass.set_bind_group(2, self.shadow.sample_bind(), &[]);
                self.draw_mesh_cmds(&mut pass, &list.meshes_xlu, &list.view_proj);
            }
            if !list.tex_meshes_xlu.is_empty() {
                self.tex_mesh.draw_xlu(&mut pass, &self.queue, list, &self.lights_bind, self.shadow.sample_bind())?;
            }
        }

        if !list.tex_meshes_emissive.is_empty() || !list.meshes_emissive.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d-emissive"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            if !list.meshes_emissive.is_empty() {
                pass.set_pipeline(&self.mesh_emissive_pipeline);
                self.draw_mesh_cmds(&mut pass, &list.meshes_emissive, &list.view_proj);
            }
            if !list.tex_meshes_emissive.is_empty() {
                self.tex_mesh.draw_emissive(&mut pass, &self.queue, list, &self.lights_bind)?;
            }
        }

        // View-model：清深度后绘制，避免世界近景裁切手臂/武器。
        if !list.view_model_meshes.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d-view-model"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.mesh_pipeline);
            pass.set_bind_group(1, &self.lights_bind, &[]);
            pass.set_bind_group(2, self.shadow.sample_bind(), &[]);
            self.draw_mesh_cmds(&mut pass, &list.view_model_meshes, &list.view_proj);
        }

        // 场景色 → bloom → 交换链；HUD 叠在交换链上保持清晰。
        if use_bloom {
            self.bloom.apply(&mut encoder, &self.queue, &view, list.bloom_strength);
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("hud"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
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
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
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
    /// 上一帧实测（供下一帧 `FrameCtx` 与慢帧日志）。
    timing: spark_renderer::FrameTiming,
    frame_index: u64,
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
                tracing::error!(event = "spark.renderer.window_create_failed", ?e);
                event_loop.exit();
                return;
            }
        };
        self.scale = window.scale_factor() as f32;
        match pollster::block_on(GpuState3d::new(window)) {
            Ok(s) => self.state = Some(s),
            Err(e) => {
                tracing::error!(event = "spark.renderer.gpu3d_init_failed", ?e);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
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
                    self.input.on_key(k, winit_map::button_state(event.state));
                }
                if event.state == winit::event::ElementState::Pressed {
                    if let Some(text) = event.text.as_ref() {
                        if !text.is_empty() && !text.chars().any(|c| c.is_control()) {
                            self.input.on_text(text.as_str());
                        }
                    }
                }
            }
            WindowEvent::Ime(ime) => match ime {
                winit::event::Ime::Enabled | winit::event::Ime::Disabled => {}
                winit::event::Ime::Preedit(text, cursor) => {
                    self.input.on_ime_preedit(text, cursor);
                }
                winit::event::Ime::Commit(text) => {
                    self.input.on_ime_commit(text);
                }
            },
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(b) = winit_map::mouse_btn(button) {
                    self.input.on_mouse_button(b, winit_map::button_state(state));
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.on_wheel(winit_map::wheel_lines(delta));
            }
            WindowEvent::CursorMoved { position, .. } => {
                // 与 surface / FrameCtx.screen_* 一致：物理像素（不再除以 scale）。
                self.input.on_cursor(position.x as f32, position.y as f32);
            }
            WindowEvent::ModifiersChanged(m) => {
                self.modifiers = m.state();
            }
            WindowEvent::RedrawRequested => self.frame(event_loop),
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: winit::event::DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.input.on_mouse_delta(delta.0 as f32, delta.1 as f32);
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
        let Some(gpu) = self.state.as_mut()
        else {
            return;
        };

        let want_grab = self.host.cursor_grab();
        if want_grab != self.grab_applied {
            if want_grab {
                let _ = gpu.window.set_cursor_grab(CursorGrabMode::Locked).or_else(|_| gpu.window.set_cursor_grab(CursorGrabMode::Confined));
                gpu.window.set_cursor_visible(false);
            }
            else {
                let _ = gpu.window.set_cursor_grab(CursorGrabMode::None);
                gpu.window.set_cursor_visible(true);
            }
            self.grab_applied = want_grab;
        }

        let now = Instant::now();
        let raw_sec = (now - self.last).as_secs_f32();
        self.last = now;
        // 模拟步长仍钳制，避免螺旋死亡；遥测用 raw_sec。
        let dt = raw_sec.clamp(1.0 / 240.0, 0.05);
        let sw = gpu.config.width as f32;
        let sh = gpu.config.height as f32;
        let prev_timing = self.timing;

        let t_update = Instant::now();
        {
            let frame = FrameCtx { input: &self.input, dt, screen_w: sw, screen_h: sh, dpi_scale: self.scale.max(0.01), timing: prev_timing };
            self.host.update(&frame);
        }
        let update_ms = t_update.elapsed().as_secs_f32() * 1000.0;
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
        let t_draw = Instant::now();
        let mut draw = DrawList3d::new(clear, spark_geometry::Mat4::IDENTITY);
        self.host.draw(&mut draw);
        let draw_ms = t_draw.elapsed().as_secs_f32() * 1000.0;

        let t_render = Instant::now();
        if let Err(e) = gpu.render(&draw) {
            tracing::error!(event = "spark.renderer.render3d_failed", ?e);
            event_loop.exit();
            return;
        }
        let render_ms = t_render.elapsed().as_secs_f32() * 1000.0;

        self.timing = spark_renderer::FrameTiming { frame_sec: raw_sec, update_ms, draw_ms, render_ms };
        self.frame_index = self.frame_index.wrapping_add(1);
        let frame_ms = raw_sec * 1000.0;
        if frame_ms > 33.0 || self.frame_index % 120 == 0 {
            tracing::info!(
                target: "spark.frame",
                frame = self.frame_index,
                frame_ms = format!("{frame_ms:.1}"),
                update_ms = format!("{update_ms:.1}"),
                draw_ms = format!("{draw_ms:.1}"),
                render_ms = format!("{render_ms:.1}"),
                "frame timing"
            );
        }

        gpu.window.request_redraw();
        let _ = DrawList::new(clear);
        let _ = self.modifiers;
    }
}

/// 窗口事件泵 + GPU 提交（3D）。帧相位编排请走 `spark_engine::run_game_3d`。
pub fn run_window_3d<H: GameHost3d + 'static>(config: WindowConfig, host: H) -> Result<(), SparkError> {
    let event_loop = EventLoop::new().map_err(|e| SparkError::new(codes::gpu_event_loop()).caused_by(e))?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = HostApp3d {
        config,
        host,
        input: Input::default(),
        state: None,
        last: Instant::now(),
        timing: spark_renderer::FrameTiming::default(),
        frame_index: 0,
        scale: 1.0,
        grab_applied: false,
        modifiers: ModifiersState::default(),
    };
    event_loop.run_app(&mut app).map_err(|e| SparkError::new(codes::gpu_event_loop()).caused_by(e))
}
