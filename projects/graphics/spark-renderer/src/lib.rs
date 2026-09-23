//! Spark 渲染抽象层：绘制列表、帧上下文与宿主契约。
//!
//! **不含** GPU / 窗口后端。桌面 wgpu 实现见 `spark-renderer-wgpu`。
//! 游戏与 `spark-widget` 只依赖本 crate 的 `DrawList` / `WindowPump2d` 等类型。

#![forbid(missing_docs)]
mod camera2d;
mod camera3d;
mod draw;
mod draw3d;
mod frustum;
mod hud_canvas;
mod particles;
mod texture;
mod texture_cache;
mod ui_batch;

pub use camera2d::Camera2d;
pub use camera3d::Camera3d;
pub use draw::{DrawLayer2d, DrawList, QuadCmd, TexQuadCmd, TextCmd};
pub use draw3d::{
    DrawList3d, FrameLights3d, MAX_SHADOW_CASCADES, MAX_SKIN_JOINTS, MeshCmd, MeshId, MeshResidentKey, MeshVertex, ShadowParams3d,
    SkinnedMeshCmd, SkinnedVertex, TexMeshCmd, TexMeshVertex,
};
pub use frustum::{CullParams, Frustum};
pub use hud_canvas::HudCanvas;
pub use particles::{Particle2d, ParticlePool2d};
pub use spark_geometry::{Aabb3, Mat4, Vec3};
pub use spark_input::{ButtonState, Input, Key, MouseBtn};
pub use spark_texture::{
    AddressMode, AlphaMode, AtlasMetadata, ColorSpace, CpuCopyPolicy, DeviceCaps, FilterMode, MipmapPolicy, Residency, SamplerDesc,
    SpriteRegion, TextureData, TextureDesc, TextureDimension, TextureFormat, TextureInfo, TextureLayout, TextureState, TextureUpload,
    TextureUsage, UploadPolicy,
};
pub use texture::{TextureId, alloc_texture_id};
pub use texture_cache::TextureCache;
pub use ui_batch::UiRenderBatch;

/// 启动窗口配置（后端无关字段）。
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// 窗口标题。
    pub title: String,
    /// 初始客户区宽度（物理像素）。
    pub width: u32,
    /// 初始客户区高度（物理像素）。
    pub height: u32,
    /// 默认清屏色（RGBA，线性浮点）。
    pub clear_color: [f64; 4],
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { title: "Spark".into(), width: 1280, height: 720, clear_color: [0.05, 0.06, 0.10, 1.0] }
    }
}

/// 上一帧宿主侧耗时（秒 / 毫秒）。`dt` 仍为模拟用钳制步长。
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameTiming {
    /// 墙钟帧间隔（秒，**未**钳制）。
    pub frame_sec: f32,
    /// 宿主 `update` 耗时（毫秒）。
    pub update_ms: f32,
    /// 宿主 `draw` 填充绘制列表耗时（毫秒）。
    pub draw_ms: f32,
    /// GPU 提交路径（含编码 + submit/present 等待）。
    pub render_ms: f32,
}

/// 每帧输入与时间。
///
/// **坐标契约（2D / 3D HUD）**：`screen_w` / `screen_h` 与 [`Input::mouse_pos`] 均为
/// **物理像素**，与 wgpu surface / `DrawList` 一致。`dpi_scale` 为窗口 `scale_factor`
/// （如 1.0 / 1.5 / 2.0），供 UI 度量或诊断；不得与上述物理坐标混用另一套空间。
pub struct FrameCtx<'a> {
    /// 本帧只读输入快照。
    pub input: &'a Input,
    /// 模拟步长（秒，通常已钳制）。
    pub dt: f32,
    /// 帧缓冲宽（物理像素）。
    pub screen_w: f32,
    /// 帧缓冲高（物理像素）。
    pub screen_h: f32,
    /// 窗口 DPI 缩放（`winit` `scale_factor`）。
    pub dpi_scale: f32,
    /// 上一帧实测耗时；首帧或未测量后端为零。
    pub timing: FrameTiming,
}

impl FrameCtx<'_> {
    /// 逻辑尺寸 = 物理尺寸 / `dpi_scale`（诊断与逻辑布局换算用）。
    pub fn logical_size(&self) -> (f32, f32) {
        let s = self.dpi_scale.max(0.01);
        (self.screen_w / s, self.screen_h / s)
    }
}

/// 2D 窗口泵契约：winit 事件循环每帧调用，**不含**游戏玩法语义。
///
/// 游戏权威在 `spark_engine::SparkRuntime`；[`RuntimeHost2d`] 实现本 trait。
/// 编辑器壳（如 Spark Studio）可实现本 trait 驱动 UI，但不得把玩法状态挂在 pump struct 字段上。
pub trait WindowPump2d {
    /// 仿真相位：输入、固定/可变步、脚本域等。勿在此提交 GPU。
    fn simulate(&mut self, frame: &FrameCtx<'_>);
    /// 世界层 present：填充 [`DrawList`]（含纹理上传）。
    fn present_world(&mut self, draw: &mut DrawList);
    /// UI present：单独批次，与世界层分开提交。默认空实现。
    fn present_ui(&mut self, ui: &mut UiRenderBatch) {}
    /// 返回 `true` 时窗口泵应退出。默认永不退出。
    fn should_exit(&self) -> bool {
        false
    }
    /// 是否显示操作系统光标。菜单自绘光标时应返回 `false`。默认可见。
    fn cursor_visible(&self) -> bool {
        true
    }
    /// 是否请求指针锁定（双摇杆鼠标瞄准）。默认不锁定。
    fn cursor_grab(&self) -> bool {
        false
    }
}

/// 3D 窗口泵契约：事件循环每帧调用 simulate + present。
pub trait WindowPump3d {
    /// 仿真相位（相机、物理等）。
    fn simulate(&mut self, frame: &FrameCtx<'_>);
    /// 3D / HUD present。
    fn present(&mut self, draw: &mut DrawList3d);
    /// 返回 `true` 时窗口泵应退出。
    fn should_exit(&self) -> bool {
        false
    }
    /// 是否请求指针锁定（第一人称）。默认锁定。
    fn cursor_grab(&self) -> bool {
        true
    }
}
