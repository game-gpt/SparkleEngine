//! 3D 绘制列表（顶点色 / 纹理三角网格 + HUD + 驻留键 / 裁剪）。

use std::sync::Arc;

use spark_core::{Color, SparkError};
use spark_geometry::{Aabb3, Mat4, Vec3};

use crate::draw::DrawList;
use crate::frustum::{CullParams, Frustum};
use crate::texture::{alloc_texture_id, RgbaImage, TextureId};

/// 驻留网格标识（跨帧稳定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshId(pub u64);

/// GPU 驻留键：`id` 稳定，`revision` 在 CPU 顶点变更时递增。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshResidentKey {
    pub id: MeshId,
    pub revision: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MeshVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}

impl MeshVertex {
    /// 无显式法线时默认朝上（天空等不参与光照的网格可忽略）。
    pub fn new(x: f32, y: f32, z: f32, color: Color) -> Self {
        Self::with_normal(x, y, z, 0.0, 1.0, 0.0, color)
    }

    pub fn with_normal(x: f32, y: f32, z: f32, nx: f32, ny: f32, nz: f32, color: Color) -> Self {
        Self {
            pos: [x, y, z],
            normal: [nx, ny, nz],
            color: color.to_array(),
        }
    }
}

/// 带 UV / 法线的纹理网格顶点。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TexMeshVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl TexMeshVertex {
    /// 无显式法线时默认朝上。
    pub fn new(x: f32, y: f32, z: f32, u: f32, v: f32, color: Color) -> Self {
        Self::with_normal(x, y, z, 0.0, 1.0, 0.0, u, v, color)
    }

    pub fn with_normal(
        x: f32,
        y: f32,
        z: f32,
        nx: f32,
        ny: f32,
        nz: f32,
        u: f32,
        v: f32,
        color: Color,
    ) -> Self {
        Self {
            pos: [x, y, z],
            normal: [nx, ny, nz],
            uv: [u, v],
            color: color.to_array(),
        }
    }
}

/// 每帧不透明前向光照参数（游戏填权威太阳方向与雾色）。
#[derive(Debug, Clone, Copy)]
pub struct FrameLights3d {
    /// 从表面指向太阳的单位方向。
    pub sun_dir: Vec3,
    pub sun_color: Color,
    pub ambient: Color,
    pub fog_color: Color,
    pub fog_density: f32,
    pub eye: Vec3,
}

impl Default for FrameLights3d {
    fn default() -> Self {
        Self {
            sun_dir: Vec3::new(0.35, 0.85, 0.25).normalized(),
            sun_color: Color::rgb(1.0, 0.92, 0.78),
            ambient: Color::rgb(0.22, 0.26, 0.34),
            fog_color: Color::rgb(0.62, 0.70, 0.82),
            fog_density: 0.0012,
            eye: Vec3::ZERO,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeshCmd {
    pub model: Mat4,
    pub vertices: Arc<[MeshVertex]>,
    pub resident: Option<MeshResidentKey>,
    pub local_aabb: Option<Aabb3>,
}

impl MeshCmd {
    pub fn world_aabb(&self) -> Option<Aabb3> {
        self.local_aabb.map(|a| a.transformed(self.model))
    }
}

#[derive(Debug, Clone)]
pub struct TexMeshCmd {
    pub model: Mat4,
    pub texture: TextureId,
    pub vertices: Arc<[TexMeshVertex]>,
    pub resident: Option<MeshResidentKey>,
    pub local_aabb: Option<Aabb3>,
}

impl TexMeshCmd {
    pub fn world_aabb(&self) -> Option<Aabb3> {
        self.local_aabb.map(|a| a.transformed(self.model))
    }
}

/// 一帧 3D 绘制 + HUD。
///
/// 提交顺序由后端保证：`sky_meshes`（SkyPass）→ 清深度 → `meshes`/`tex_meshes`（Opaque）→ HUD。
/// 透明 / 自发光 / 后处理列表尚未立契约，禁止把半透明语义塞进不透明网格。
#[derive(Debug)]
pub struct DrawList3d {
    pub clear: Color,
    pub view_proj: Mat4,
    /// 天空 / 天体专用 VP（通常为去平移的 `Camera3d::sky_view_proj`）。
    pub sky_view_proj: Mat4,
    /// 不透明前向光照（天空 pass 不消费）。
    pub lights: FrameLights3d,
    /// SkyPass：无深度写入；在清深度前绘制。
    pub sky_meshes: Vec<MeshCmd>,
    pub meshes: Vec<MeshCmd>,
    pub tex_meshes: Vec<TexMeshCmd>,
    /// 本帧新建 / 更新纹理，由 wgpu 后端上传。
    pub texture_uploads: Vec<(TextureId, RgbaImage)>,
    pub hud: DrawList,
}

impl DrawList3d {
    pub fn new(clear: Color, view_proj: Mat4) -> Self {
        Self {
            clear,
            view_proj,
            sky_view_proj: view_proj,
            lights: FrameLights3d::default(),
            sky_meshes: Vec::new(),
            meshes: Vec::new(),
            tex_meshes: Vec::new(),
            texture_uploads: Vec::new(),
            hud: DrawList::new(Color::rgba(0.0, 0.0, 0.0, 0.0)),
        }
    }

    /// 分配稳定纹理 ID 并排队上传。请缓存返回的 ID，勿每帧为同一贴图重复分配。
    pub fn create_texture(
        &mut self,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    ) -> Result<TextureId, SparkError> {
        let img = RgbaImage::from_rgba8(width, height, rgba)?;
        let id = alloc_texture_id();
        self.texture_uploads.push((id, img));
        Ok(id)
    }

    /// 用已有 ID 重新上传像素（热重载 / 图集更新）。
    pub fn update_texture(
        &mut self,
        id: TextureId,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    ) -> Result<(), SparkError> {
        let img = RgbaImage::from_rgba8(width, height, rgba)?;
        self.texture_uploads.push((id, img));
        Ok(())
    }

    pub fn mesh(&mut self, model: Mat4, vertices: Arc<[MeshVertex]>) {
        self.push_mesh(model, vertices, None, None);
    }

    pub fn mesh_vec(&mut self, model: Mat4, vertices: Vec<MeshVertex>) {
        if !vertices.is_empty() {
            self.mesh(model, Arc::<[MeshVertex]>::from(vertices));
        }
    }

    pub fn mesh_culled(&mut self, model: Mat4, vertices: Arc<[MeshVertex]>, local_aabb: Aabb3) {
        self.push_mesh(model, vertices, None, Some(local_aabb));
    }

    pub fn mesh_resident(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
    ) {
        self.push_mesh(model, vertices, Some(key), local_aabb);
    }

    /// 天空 / 天体网格（走 SkyPass，不参与不透明深度竞争）。
    pub fn sky_mesh(&mut self, model: Mat4, vertices: Arc<[MeshVertex]>) {
        self.push_sky_mesh(model, vertices, None, None);
    }

    pub fn sky_mesh_resident(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
    ) {
        self.push_sky_mesh(model, vertices, Some(key), local_aabb);
    }

    pub fn tex_mesh(
        &mut self,
        model: Mat4,
        texture: TextureId,
        vertices: Arc<[TexMeshVertex]>,
    ) {
        self.push_tex_mesh(model, texture, vertices, None, None);
    }

    pub fn tex_mesh_resident(
        &mut self,
        model: Mat4,
        texture: TextureId,
        vertices: Arc<[TexMeshVertex]>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
    ) {
        self.push_tex_mesh(model, texture, vertices, Some(key), local_aabb);
    }

    fn push_mesh(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        resident: Option<MeshResidentKey>,
        local_aabb: Option<Aabb3>,
    ) {
        if vertices.is_empty() {
            return;
        }
        self.meshes.push(MeshCmd {
            model,
            vertices,
            resident,
            local_aabb,
        });
    }

    fn push_sky_mesh(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        resident: Option<MeshResidentKey>,
        local_aabb: Option<Aabb3>,
    ) {
        if vertices.is_empty() {
            return;
        }
        self.sky_meshes.push(MeshCmd {
            model,
            vertices,
            resident,
            local_aabb,
        });
    }

    fn push_tex_mesh(
        &mut self,
        model: Mat4,
        texture: TextureId,
        vertices: Arc<[TexMeshVertex]>,
        resident: Option<MeshResidentKey>,
        local_aabb: Option<Aabb3>,
    ) {
        if vertices.is_empty() {
            return;
        }
        self.tex_meshes.push(TexMeshCmd {
            model,
            texture,
            vertices,
            resident,
            local_aabb,
        });
    }

    pub fn retain_visible(&mut self, cull: CullParams) {
        let frustum = Frustum::from_view_proj(&self.view_proj);
        let keep = |world: Aabb3| -> bool {
            if let Some(max_d) = cull.max_distance {
                let c = world.center();
                let dist = (c - cull.eye).length() - world.extents().length();
                if dist > max_d {
                    return false;
                }
            }
            frustum.intersects_aabb(&world)
        };
        self.meshes.retain(|m| m.world_aabb().map(keep).unwrap_or(true));
        self.tex_meshes
            .retain(|m| m.world_aabb().map(keep).unwrap_or(true));
    }

    pub fn retain_within_distance(&mut self, eye: Vec3, max_distance: f32) {
        let max_d = max_distance.max(0.0);
        let keep = |world: Aabb3| -> bool {
            let c = world.center();
            let dist = (c - eye).length() - world.extents().length();
            dist <= max_d
        };
        self.meshes.retain(|m| m.world_aabb().map(keep).unwrap_or(true));
        self.tex_meshes
            .retain(|m| m.world_aabb().map(keep).unwrap_or(true));
    }
}
