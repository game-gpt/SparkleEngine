//! Spark 渲染抽象层：绘制列表、帧上下文与宿主契约。
//!
//! **不含** GPU / 窗口后端。桌面 wgpu 实现见 `spark-renderer-wgpu`。
//! 游戏与 `spark-widget` 只依赖本 crate 的 `DrawList` / `GameHost` 等类型。

#![warn(missing_docs)]
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
pub use texture::{TextureId, alloc_texture_id};
pub use texture_cache::TextureCache;
pub use spark_texture::{
    AddressMode, AlphaMode, AtlasMetadata, ColorSpace, CpuCopyPolicy, DeviceCaps, FilterMode, MipmapPolicy, Residency, SamplerDesc,
    SpriteRegion, TextureData, TextureDesc, TextureDimension, TextureFormat, TextureInfo, TextureLayout, TextureState, TextureUpload,
    TextureUsage, UploadPolicy,
};

/// 启动窗口配置（后端无关字段）。
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
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
    pub update_ms: f32,
    pub draw_ms: f32,
    /// GPU 提交路径（含编码 + submit/present 等待）。
    pub render_ms: f32,
}

/// 每帧输入与时间。
pub struct FrameCtx<'a> {
    pub input: &'a Input,
    pub dt: f32,
    pub screen_w: f32,
    pub screen_h: f32,
    /// 上一帧实测耗时；首帧或未测量后端为零。
    pub timing: FrameTiming,
}

/// 2D 游戏宿主：更新逻辑并填充绘制列表。
pub trait GameHost {
    fn update(&mut self, frame: &FrameCtx<'_>);
    fn draw(&mut self, draw: &mut DrawList);
    fn should_exit(&self) -> bool {
        false
    }
}

/// 3D 游戏宿主：透视网格 + 可选 2D HUD。
pub trait GameHost3d {
    fn update(&mut self, frame: &FrameCtx<'_>);
    fn draw(&mut self, draw: &mut DrawList3d);
    fn should_exit(&self) -> bool {
        false
    }
    /// 是否请求指针锁定（第一人称）。
    fn cursor_grab(&self) -> bool {
        true
    }
}
