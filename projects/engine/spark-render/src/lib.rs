//! 最小窗口与清屏循环（wgpu）。
//!
//! 游戏层只提供标题与清屏色；不包含玩法语义。

use std::sync::Arc;

use spark_core::SparkError;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

/// 启动窗口的配置。
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// sRGB 清屏色，分量范围 0.0–1.0。
    pub clear_color: [f64; 4],
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Spark".into(),
            width: 1280,
            height: 720,
            clear_color: [0.05, 0.06, 0.10, 1.0],
        }
    }
}

struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    clear: wgpu::Color,
}

impl GpuState {
    async fn new(window: Arc<Window>, clear: wgpu::Color) -> Result<Self, SparkError> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .map_err(|err| SparkError::Message(format!("创建 surface 失败：{err}")))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|err| SparkError::Message(format!("request adapter 失败：{err}")))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("spark-render-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: Default::default(),
            })
            .await
            .map_err(|err| SparkError::Message(format!("request device 失败：{err}")))?;

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
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            clear,
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

    fn redraw(&mut self) -> Result<(), SparkError> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(err) => {
                return Err(SparkError::Message(format!("取得 surface texture 失败：{err}")));
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("spark-render-encoder"),
            });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("spark-clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.window.pre_present_notify();
        frame.present();
        Ok(())
    }
}

struct App {
    title: String,
    width: u32,
    height: u32,
    clear: wgpu::Color,
    state: Option<GpuState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title(self.title.clone())
            .with_inner_size(LogicalSize::new(self.width, self.height));

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(err) => {
                tracing::error!(?err, "创建窗口失败");
                event_loop.exit();
                return;
            }
        };

        match pollster::block_on(GpuState::new(window.clone(), self.clear)) {
            Ok(state) => {
                tracing::info!(title = %self.title, "窗口与 GPU 已就绪");
                window.request_redraw();
                self.state = Some(state);
            }
            Err(err) => {
                tracing::error!(?err, "GPU 初始化失败");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("收到关闭请求");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                state.resize(size.width, size.height);
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let Err(err) = state.redraw() {
                    tracing::error!(?err, "绘制失败");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }
    }
}

/// 阻塞运行窗口，直到用户关闭。
pub fn run_window(config: WindowConfig) -> Result<(), SparkError> {
    let clear = wgpu::Color {
        r: config.clear_color[0],
        g: config.clear_color[1],
        b: config.clear_color[2],
        a: config.clear_color[3],
    };

    let event_loop = EventLoop::new().map_err(|err| SparkError::Message(err.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        title: config.title,
        width: config.width.max(1),
        height: config.height.max(1),
        clear,
        state: None,
    };

    event_loop
        .run_app(&mut app)
        .map_err(|err| SparkError::Message(err.to_string()))
}
