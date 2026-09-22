//! Spark 渲染抽象层：绘制列表、帧上下文与宿主契约。
//!
//! **不含** GPU / 窗口后端。桌面 wgpu 实现见 `spark-renderer-wgpu`。
//! 游戏与 `spark-widget` 只依赖本 crate 的 `DrawList` / `GameHost` 等类型。

#![forbid(missing_docs)]
mod camera2d;
mod camera3d;
mod draw;
mod draw3d;
mod frustum;
mod particles;
mod texture;
mod texture_cache;

pub use camera2d::Camera2d;
pub use camera3d::Camera3d;
pub use draw::{DrawLayer2d, DrawList, QuadCmd, TexQuadCmd, TextCmd};
pub use draw3d::{
    DrawList3d, FrameLights3d, MAX_SHADOW_CASCADES, MAX_SKIN_JOINTS, MeshCmd, MeshId, MeshResidentKey, MeshVertex, ShadowParams3d,
    SkinnedMeshCmd, SkinnedVertex, TexMeshCmd, TexMeshVertex,
};
pub use frustum::{CullParams, Frustum};
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

/// 2D 游戏宿主：更新逻辑并填充绘制列表。
pub trait GameHost {
    /// 每帧逻辑更新（输入、模拟）；勿在此提交 GPU。
    fn update(&mut self, frame: &FrameCtx<'_>);
    /// 将本帧 2D 命令写入 [`DrawList`]（世界 / HUD / 纹理上传）。
    fn draw(&mut self, draw: &mut DrawList);
    /// 返回 `true` 时宿主循环应退出。默认永不退出。
    fn should_exit(&self) -> bool {
        false
    }
    /// 是否显示操作系统光标。菜单自绘光标时应返回 `false`。默认可见。
    fn cursor_visible(&self) -> bool {
        true
    }
}

/// 3D 游戏宿主：透视网格 + 可选 2D HUD。
pub trait GameHost3d {
    /// 每帧逻辑更新（相机、模拟）；勿在此提交 GPU。
    fn update(&mut self, frame: &FrameCtx<'_>);
    /// 将本帧 3D / HUD 命令写入 [`DrawList3d`]。
    fn draw(&mut self, draw: &mut DrawList3d);
    /// 返回 `true` 时宿主循环应退出。默认永不退出。
    fn should_exit(&self) -> bool {
        false
    }
    /// 是否请求指针锁定（第一人称）。
    fn cursor_grab(&self) -> bool {
        true
    }
}
