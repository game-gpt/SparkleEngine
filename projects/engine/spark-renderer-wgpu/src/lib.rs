//! Spark 渲染 **wgpu** 后端：窗口壳与批绘制提交。
//!
//! 抽象类型（`DrawList` / `GameHost` / `FrameCtx` / `WindowConfig`）在 `spark-renderer`。
//! **winit 止于此 crate**：游戏只看见 `spark-renderer` / `spark-input` 类型。

mod winit_map;

pub use spark_font::{GlyphCache, GlyphInfo};
pub use spark_renderer::{
    ButtonState, DrawList, FrameCtx, GameHost, Input, Key, MouseBtn, QuadCmd, TextCmd, WindowConfig,
};

use std::sync::Arc;
use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use spark_core::{Color, SparkError};
use spark_shader::{BuiltinShader, create_builtin};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
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

struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    solid_pipeline: wgpu::RenderPipeline,
    glyph_pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    solid_bind: wgpu::BindGroup,
    glyph_bind: wgpu::BindGroup,
    glyph_tex: wgpu::Texture,
    glyph_view: wgpu::TextureView,
    glyph_cache: GlyphCache,
    solid_vbo: wgpu::Buffer,
    glyph_vbo: wgpu::Buffer,
    solid_cap: u64,
    glyph_cap: u64,
}

impl GpuState {
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
            .map_err(|e| SparkError::Message(format!("adapter: {e}")))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("spark-renderer-wgpu"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                memory_hints: Default::default(),
                trace: Default::default(),
            })
            .await
            .map_err(|e| SparkError::Message(format!("device: {e}")))?;

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

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let solid_shader = create_builtin(&device, BuiltinShader::SolidQuad);
        let glyph_shader = create_builtin(&device, BuiltinShader::TexturedGlyph);

        let solid_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("solid-bgl"),
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
        let solid_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("solid-bg"),
            layout: &solid_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let mut glyph_cache = GlyphCache::load_system()?;
        let (aw, ah) = glyph_cache.atlas_size();
        let glyph_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph-atlas"),
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

        let glyph_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glyph-bgl"),
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
        let glyph_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph-bg"),
            layout: &glyph_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buf.as_entire_binding(),
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
            label: Some("solid-pl"),
            bind_group_layouts: &[Some(&solid_bgl)],
            immediate_size: 0,
        });
        let glyph_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("glyph-pl"),
            bind_group_layouts: &[Some(&glyph_bgl)],
            immediate_size: 0,
        });

        let solid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("solid-pipeline"),
            layout: Some(&solid_pl),
            vertex: wgpu::VertexState {
                module: &solid_shader,
                entry_point: Some(BuiltinShader::SolidQuad.vertex_entry()),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<SolidVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &solid_shader,
                entry_point: Some(BuiltinShader::SolidQuad.fragment_entry()),
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

        let glyph_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("glyph-pipeline"),
            layout: Some(&glyph_pl),
            vertex: wgpu::VertexState {
                module: &glyph_shader,
                entry_point: Some(BuiltinShader::TexturedGlyph.vertex_entry()),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GlyphVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &glyph_shader,
                entry_point: Some(BuiltinShader::TexturedGlyph.fragment_entry()),
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

        // 侧视方块竖切：约 70×40 可见格 × 6 顶点，再留 HUD / 余量。
        let solid_cap = 65536u64;
        let glyph_cap = 32768u64;
        let solid_vbo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("solid-vbo"),
            size: solid_cap * std::mem::size_of::<SolidVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let glyph_vbo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glyph-vbo"),
            size: glyph_cap * std::mem::size_of::<GlyphVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            solid_pipeline,
            glyph_pipeline,
            uniform_buf,
            solid_bind,
            glyph_bind,
            glyph_tex,
            glyph_view,
            glyph_cache,
            solid_vbo,
            glyph_vbo,
            solid_cap,
            glyph_cap,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
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
            label: Some("solid-vbo"),
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
            label: Some("glyph-vbo"),
            size: cap * std::mem::size_of::<GlyphVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.glyph_cap = cap;
        Ok(())
    }

    fn upload_atlas_if_needed(&mut self) {
        if !self.glyph_cache.take_dirty() {
            return;
        }
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
        let _ = &self.glyph_view;
    }

    fn render(&mut self, list: &DrawList) -> Result<(), SparkError> {
        let sw = self.config.width as f32;
        let sh = self.config.height as f32;
        self.queue.write_buffer(
            &self.uniform_buf,
            0,
            bytemuck::bytes_of(&Uniforms {
                screen: [sw, sh],
                _pad: [0.0, 0.0],
            }),
        );

        let mut solids = Vec::with_capacity(list.quads.len() * 6);
        for q in &list.quads {
            let x0 = q.rect.x;
            let y0 = q.rect.y;
            let x1 = q.rect.x + q.rect.w;
            let y1 = q.rect.y + q.rect.h;
            let c = q.color.to_array();
            let verts = [
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
            ];
            solids.extend_from_slice(&verts);
        }

        let mut glyphs = Vec::new();
        for t in &list.texts {
            let mut pen_x = t.pos.x;
            let baseline = t.pos.y + t.size;
            for ch in t.text.chars() {
                if ch == '\n' {
                    continue;
                }
                let Some((advance, width, height, bearing_y, u0, v0, u1, v1)) =
                    self.glyph_cache.glyph(ch, t.size).map(|g| {
                        (
                            g.advance,
                            g.width,
                            g.height,
                            g.bearing_y,
                            g.uv_min[0],
                            g.uv_min[1],
                            g.uv_max[0],
                            g.uv_max[1],
                        )
                    })
                else {
                    continue;
                };
                let x0 = pen_x;
                let y0 = baseline - bearing_y - height;
                let x1 = x0 + width;
                let y1 = y0 + height;
                let c = t.color.to_array();
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
                pen_x += advance;
            }
        }
        self.upload_atlas_if_needed();

        self.ensure_solid_cap(solids.len() as u64)?;
        self.ensure_glyph_cap(glyphs.len() as u64)?;
        if !solids.is_empty() {
            self.queue
                .write_buffer(&self.solid_vbo, 0, bytemuck::cast_slice(&solids));
        }
        if !glyphs.is_empty() {
            self.queue
                .write_buffer(&self.glyph_vbo, 0, bytemuck::cast_slice(&glyphs));
        }

        let (frame, suboptimal) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex) => (tex, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(tex) => (tex, true),
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                return Ok(());
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
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
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
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
        self.queue.submit(std::iter::once(encoder.finish()));
        self.window.pre_present_notify();
        self.queue.present(frame);
        if suboptimal {
            self.surface.configure(&self.device, &self.config);
        }
        Ok(())
    }
}

struct HostApp<H: GameHost> {
    config: WindowConfig,
    host: H,
    input: Input,
    state: Option<GpuState>,
    last: Instant,
    scale: f32,
}

impl<H: GameHost> ApplicationHandler for HostApp<H> {
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
                tracing::error!(?e, "create window");
                event_loop.exit();
                return;
            }
        };
        self.scale = window.scale_factor() as f32;
        match pollster::block_on(GpuState::new(window.clone())) {
            Ok(s) => {
                tracing::info!("GPU ready");
                window.request_redraw();
                self.state = Some(s);
                self.last = Instant::now();
            }
            Err(e) => {
                tracing::error!(?e, "GPU init failed");
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
        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
            }
            WindowEvent::CursorMoved { position, .. } => {
                // 与 surface / uniform 一致：物理像素
                self.input
                    .on_cursor(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(btn) = winit_map::mouse_btn(*button) {
                    self.input
                        .on_mouse_button(btn, winit_map::button_state(*state));
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(key) = winit_map::key(event.physical_key) {
                    self.input
                        .on_key(key, winit_map::button_state(event.state));
                }
            }
            _ => {}
        }

        let Some(gpu) = self.state.as_mut() else {
            return;
        };

        if let WindowEvent::Resized(size) = event {
            gpu.resize(size.width, size.height);
            gpu.window.request_redraw();
            return;
        }

        if !matches!(event, WindowEvent::RedrawRequested) {
            return;
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
        let mut draw = DrawList::new(clear);
        self.host.draw(&mut draw);
        if let Err(e) = gpu.render(&draw) {
            tracing::error!(?e, "render failed");
            event_loop.exit();
            return;
        }
        gpu.window.request_redraw();
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        _event: DeviceEvent,
    ) {
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(s) = self.state.as_ref() {
            s.window.request_redraw();
        }
    }
}

/// 阻塞运行游戏宿主，直到退出。
pub fn run_game<H: GameHost + 'static>(config: WindowConfig, host: H) -> Result<(), SparkError> {
    let event_loop = EventLoop::new().map_err(|e| SparkError::Message(e.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = HostApp {
        config,
        host,
        input: Input::default(),
        state: None,
        last: Instant::now(),
        scale: 1.0,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|e| SparkError::Message(e.to_string()))
}

/// 仅清屏窗口（无宿主逻辑）。
pub fn run_window(config: WindowConfig) -> Result<(), SparkError> {
    struct Empty;
    impl GameHost for Empty {
        fn update(&mut self, _: &FrameCtx<'_>) {}
        fn draw(&mut self, _: &mut DrawList) {}
    }
    run_game(config, Empty)
}
