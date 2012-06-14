//! Spark 渲染抽象层：绘制列表、帧上下文与宿主契约。
//!
//! **不含** GPU / 窗口后端。桌面 wgpu 实现见 `spark-renderer-wgpu`。
//! 游戏与 `spark-widget` 只依赖本 crate 的 `DrawList` / `GameHost` 等类型。

mod camera3d;
mod draw;
mod draw3d;
mod frustum;
mod texture;

pub use camera3d::Camera3d;
pub use draw::{DrawList, QuadCmd, TextCmd};
pub use draw3d::{
    DrawList3d, FrameLights3d, MeshCmd, MeshId, MeshResidentKey, MeshVertex, ShadowParams3d,
    MAX_SHADOW_CASCADES,
    SkinnedMeshCmd, SkinnedVertex, TexMeshCmd, TexMeshVertex, MAX_SKIN_JOINTS,
};
pub use frustum::{CullParams, Frustum};
pub use spark_geometry::{Aabb3, Mat4, Vec3};
pub use spark_input::{ButtonState, Input, Key, MouseBtn};
pub use texture::{alloc_texture_id, RgbaImage, TextureId};

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
        Self {
            title: "Spark".into(),
            width: 1280,
            height: 720,
            clear_color: [0.05, 0.06, 0.10, 1.0],
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use spark_core::Color;
    use std::sync::Arc;

    #[test]
    fn retain_visible_drops_far_mesh() {
        let cam = Camera3d {
            eye: Vec3::new(0.0, 0.0, 5.0),
            yaw: 0.0,
            pitch: 0.0,
            fov_y_rad: 70f32.to_radians(),
            near: 0.1,
            far: 100.0,
        };
        let mut list = DrawList3d::new(Color::rgb(0.0, 0.0, 0.0), cam.view_proj(16.0 / 9.0));
        let verts: Arc<[MeshVertex]> =
            Arc::from(vec![MeshVertex::new(0.0, 0.0, 0.0, Color::rgb(1.0, 1.0, 1.0))]);
        list.mesh_culled(
            Mat4::translation(Vec3::new(0.0, 0.0, 0.0)),
            Arc::clone(&verts),
            Aabb3::from_min_max(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5)),
        );
        list.mesh_culled(
            Mat4::translation(Vec3::new(0.0, 0.0, -500.0)),
            verts,
            Aabb3::from_min_max(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5)),
        );
        assert_eq!(list.meshes.len(), 2);
        list.retain_visible(CullParams::new(cam.eye).with_max_distance(50.0));
        assert_eq!(list.meshes.len(), 1);
    }

    #[test]
    fn create_texture_queues_upload() {
        let mut list = DrawList3d::new(Color::rgb(0.0, 0.0, 0.0), Mat4::IDENTITY);
        let id = list.create_texture(1, 1, vec![255, 0, 0, 255]).unwrap();
        assert!(id.0 >= 1);
        assert_eq!(list.texture_uploads.len(), 1);
    }

    #[test]
    fn skinned_mesh_truncates_palette() {
        let mut list = DrawList3d::new(Color::rgb(0.0, 0.0, 0.0), Mat4::IDENTITY);
        let vert = SkinnedVertex::new(
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0],
            Color::rgb(1.0, 1.0, 1.0),
            [0, 0, 0, 0],
            [1.0, 0.0, 0.0, 0.0],
        );
        let palette: Vec<Mat4> = (0..MAX_SKIN_JOINTS + 8).map(|_| Mat4::IDENTITY).collect();
        list.skinned_mesh(
            Mat4::IDENTITY,
            Arc::from(vec![vert]),
            Arc::from(palette),
            None,
        );
        assert_eq!(list.skinned_meshes.len(), 1);
        assert_eq!(list.skinned_meshes[0].joint_palette.len(), MAX_SKIN_JOINTS);
    }
}
