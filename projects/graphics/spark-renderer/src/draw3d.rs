//! 3D 绘制列表（顶点色 / 纹理 / 蒙皮三角网格 + HUD + 驻留键 / 裁剪）。

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
    /// ACES 前曝光倍率（典型 0.9–1.4）。
    pub exposure: f32,
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
            exposure: 1.15,
        }
    }
}

/// 太阳阴影级联上限（与 GPU uniform / 深度数组层数一致）。
pub const MAX_SHADOW_CASCADES: usize = 3;

/// 太阳正交阴影（1..3 级联）；`enabled=false` 时着色器跳过采样。
#[derive(Debug, Clone, Copy)]
pub struct ShadowParams3d {
    pub enabled: bool,
    /// 有效级联数，钳制到 `1..=MAX_SHADOW_CASCADES`。
    pub cascade_count: u32,
    pub light_view_proj: [Mat4; MAX_SHADOW_CASCADES],
    /// 相对 `focus`（通常为相机眼）的世界距离：级联 `i` 覆盖到 `split_end[i]`。
    pub split_end: [f32; MAX_SHADOW_CASCADES],
    pub bias: f32,
    /// 阴影压暗强度 0..1。
    pub strength: f32,
}

impl Default for ShadowParams3d {
    fn default() -> Self {
        Self {
            enabled: false,
            cascade_count: 1,
            light_view_proj: [Mat4::IDENTITY; MAX_SHADOW_CASCADES],
            split_end: [0.0; MAX_SHADOW_CASCADES],
            bias: 0.0018,
            strength: 0.55,
        }
    }
}

impl ShadowParams3d {
    /// 以 `focus` 为中心的单级太阳正交阴影盒（世界单位）。
    pub fn from_sun(sun_dir: Vec3, focus: Vec3, half_extent: f32) -> Self {
        Self::from_sun_cascaded(sun_dir, focus, &[half_extent])
    }

    /// 同焦点、多级正交盒：近景高分辨率 + 远景覆盖。
    ///
    /// `half_extents` 由近到远，长度 1..=3；空切片退化为关闭。
    pub fn from_sun_cascaded(sun_dir: Vec3, focus: Vec3, half_extents: &[f32]) -> Self {
        if half_extents.is_empty() {
            return Self::default();
        }
        let sun = sun_dir.normalized();
        let n = half_extents.len().min(MAX_SHADOW_CASCADES);
        let mut light_view_proj = [Mat4::IDENTITY; MAX_SHADOW_CASCADES];
        let mut split_end = [0.0f32; MAX_SHADOW_CASCADES];
        for i in 0..n {
            let extent = half_extents[i].max(8.0);
            let depth = extent * 3.0;
            let eye = focus + sun * (depth * 0.42);
            let view = Mat4::look_to(eye, -sun, Vec3::Y);
            let proj = Mat4::orthographic(-extent, extent, -extent, extent, 1.0, depth);
            light_view_proj[i] = proj.mul(view);
            // 盒半宽近似覆盖半径；略放大避免盒角被裁切。
            split_end[i] = extent * 1.35;
        }
        Self {
            enabled: true,
            cascade_count: n as u32,
            light_view_proj,
            split_end,
            bias: 0.0018,
            strength: 0.58,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeshCmd {
    pub model: Mat4,
    pub vertices: Arc<[MeshVertex]>,
    pub resident: Option<MeshResidentKey>,
    pub local_aabb: Option<Aabb3>,
    /// 是否参与太阳阴影深度 pass；远景代理可关以省级联开销。
    pub casts_shadow: bool,
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
    pub casts_shadow: bool,
}

impl TexMeshCmd {
    pub fn world_aabb(&self) -> Option<Aabb3> {
        self.local_aabb.map(|a| a.transformed(self.model))
    }
}

/// 蒙皮关节 palette 首切上限（与 `spark-anim::MAX_JOINTS` / WGSL uniform 一致）。
pub const MAX_SKIN_JOINTS: usize = 64;

/// 蒙皮顶点：最多 4 影响；权重应归一化到 1。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SkinnedVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub joints: [u32; 4],
    pub weights: [f32; 4],
}

impl SkinnedVertex {
    pub fn new(
        pos: [f32; 3],
        normal: [f32; 3],
        uv: [f32; 2],
        color: Color,
        joints: [u32; 4],
        weights: [f32; 4],
    ) -> Self {
        Self {
            pos,
            normal,
            uv,
            color: color.to_array(),
            joints,
            weights,
        }
    }
}

/// 不透明蒙皮网格绘制命令（Opaque；透明/自发光后置）。
#[derive(Debug, Clone)]
pub struct SkinnedMeshCmd {
    pub model: Mat4,
    pub vertices: Arc<[SkinnedVertex]>,
    /// `global * inverse_bind`；长度 ≤ [`MAX_SKIN_JOINTS`]。
    pub joint_palette: Arc<[Mat4]>,
    pub texture: Option<TextureId>,
    pub resident: Option<MeshResidentKey>,
    pub local_aabb: Option<Aabb3>,
}

impl SkinnedMeshCmd {
    pub fn world_aabb(&self) -> Option<Aabb3> {
        self.local_aabb.map(|a| a.transformed(self.model))
    }
}

/// 一帧 3D 绘制 + HUD。
///
/// 提交顺序由后端保证：`sky_atmosphere_meshes` → `sky_meshes` → `sky_emissive_meshes`
/// （additive）→ 清深度 → Opaque → Transparent → Emissive（世界加性）→
/// 清深度 → `view_model_meshes`（第一人称手臂/武器）→ bloom → HUD。
#[derive(Debug)]
pub struct DrawList3d {
    pub clear: Color,
    pub view_proj: Mat4,
    /// 天空 / 天体专用 VP（通常为去平移的 `Camera3d::sky_view_proj`）。
    pub sky_view_proj: Mat4,
    /// 不透明前向光照；大气穹顶片段着色器亦消费 `sun_dir`。
    pub lights: FrameLights3d,
    /// SkyPass 大气穹顶：视线方向散射（`SkyAtmosphere3d`）。
    pub sky_atmosphere_meshes: Vec<MeshCmd>,
    /// SkyPass：顶点色穹顶 / 天体（无深度写入）。
    pub sky_meshes: Vec<MeshCmd>,
    /// Sky 加性发光（方日光晕等）；测深 Always、不写深、additive。
    pub sky_emissive_meshes: Vec<MeshCmd>,
    pub meshes: Vec<MeshCmd>,
    /// 顶点色半透明（面罩等）；测深不写深。
    pub meshes_xlu: Vec<MeshCmd>,
    /// 顶点色自发光（灯条等）；测深不写深、additive。
    pub meshes_emissive: Vec<MeshCmd>,
    pub tex_meshes: Vec<TexMeshCmd>,
    /// Transparent：树叶 / 玻璃等；深度测试开启、不写深度。
    pub tex_meshes_xlu: Vec<TexMeshCmd>,
    /// 世界自发光（岩浆/引擎等）；测深、不写深、additive。
    pub tex_meshes_emissive: Vec<TexMeshCmd>,
    /// 不透明蒙皮网格（在静态 meshes / tex_meshes 之后绘制）。
    pub skinned_meshes: Vec<SkinnedMeshCmd>,
    /// 第一人称 view-model：世界 pass 之后清深度再画，避免被近景墙体裁切。
    pub view_model_meshes: Vec<MeshCmd>,
    /// 本帧新建 / 更新纹理，由 wgpu 后端上传。
    pub texture_uploads: Vec<(TextureId, RgbaImage)>,
    /// 全屏 bloom 强度；`0` 关闭后处理。
    pub bloom_strength: f32,
    /// 单级太阳阴影参数。
    pub shadow: ShadowParams3d,
    pub hud: DrawList,
}

impl DrawList3d {
    pub fn new(clear: Color, view_proj: Mat4) -> Self {
        Self {
            clear,
            view_proj,
            sky_view_proj: view_proj,
            lights: FrameLights3d::default(),
            sky_atmosphere_meshes: Vec::new(),
            sky_meshes: Vec::new(),
            sky_emissive_meshes: Vec::new(),
            meshes: Vec::new(),
            meshes_xlu: Vec::new(),
            meshes_emissive: Vec::new(),
            tex_meshes: Vec::new(),
            tex_meshes_xlu: Vec::new(),
            tex_meshes_emissive: Vec::new(),
            skinned_meshes: Vec::new(),
            view_model_meshes: Vec::new(),
            texture_uploads: Vec::new(),
            bloom_strength: 0.55,
            shadow: ShadowParams3d::default(),
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
        self.push_mesh(model, vertices, None, None, true);
    }

    pub fn mesh_vec(&mut self, model: Mat4, vertices: Vec<MeshVertex>) {
        if !vertices.is_empty() {
            self.mesh(model, Arc::<[MeshVertex]>::from(vertices));
        }
    }

    pub fn mesh_culled(&mut self, model: Mat4, vertices: Arc<[MeshVertex]>, local_aabb: Aabb3) {
        self.push_mesh(model, vertices, None, Some(local_aabb), true);
    }

    pub fn mesh_resident(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
    ) {
        self.push_mesh(model, vertices, Some(key), local_aabb, true);
    }

    /// 与 `mesh_resident` 相同，但可关闭阴影投射（远景高度场等）。
    pub fn mesh_resident_cast_shadow(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
        casts_shadow: bool,
    ) {
        self.push_mesh(model, vertices, Some(key), local_aabb, casts_shadow);
    }

    /// 顶点色半透明网格（Transparent pass）。
    pub fn mesh_xlu(&mut self, model: Mat4, vertices: Arc<[MeshVertex]>) {
        self.push_mesh_xlu(model, vertices, None, None);
    }

    pub fn mesh_xlu_resident(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
    ) {
        self.push_mesh_xlu(model, vertices, Some(key), local_aabb);
    }

    /// 顶点色自发光网格（Emissive pass）。
    pub fn mesh_emissive(&mut self, model: Mat4, vertices: Arc<[MeshVertex]>) {
        self.push_mesh_emissive(model, vertices, None, None);
    }

    pub fn mesh_emissive_resident(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
    ) {
        self.push_mesh_emissive(model, vertices, Some(key), local_aabb);
    }

    /// 第一人称 view-model（独立深度 pass）。
    pub fn view_model_mesh(&mut self, model: Mat4, vertices: Arc<[MeshVertex]>) {
        self.push_view_model_mesh(model, vertices, None, None);
    }

    pub fn view_model_mesh_resident(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
    ) {
        self.push_view_model_mesh(model, vertices, Some(key), local_aabb);
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

    /// 大气穹顶（视线散射）；顶点色作 tint。
    pub fn sky_atmosphere_mesh(&mut self, model: Mat4, vertices: Arc<[MeshVertex]>) {
        self.push_sky_atmosphere(model, vertices, None, None);
    }

    pub fn sky_atmosphere_mesh_resident(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
    ) {
        self.push_sky_atmosphere(model, vertices, Some(key), local_aabb);
    }

    /// 天空加性发光（方日光晕等）。
    pub fn sky_emissive_mesh(&mut self, model: Mat4, vertices: Arc<[MeshVertex]>) {
        self.push_sky_emissive(model, vertices, None, None);
    }

    pub fn sky_emissive_mesh_resident(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
    ) {
        self.push_sky_emissive(model, vertices, Some(key), local_aabb);
    }

    pub fn tex_mesh(
        &mut self,
        model: Mat4,
        texture: TextureId,
        vertices: Arc<[TexMeshVertex]>,
    ) {
        self.push_tex_mesh(model, texture, vertices, None, None, TexPass::Opaque);
    }

    pub fn tex_mesh_resident(
        &mut self,
        model: Mat4,
        texture: TextureId,
        vertices: Arc<[TexMeshVertex]>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
    ) {
        self.push_tex_mesh(model, texture, vertices, Some(key), local_aabb, TexPass::Opaque);
    }

    /// 半透明纹理网格（Transparent pass：测深不写深）。
    pub fn tex_mesh_xlu(
        &mut self,
        model: Mat4,
        texture: TextureId,
        vertices: Arc<[TexMeshVertex]>,
    ) {
        self.push_tex_mesh(model, texture, vertices, None, None, TexPass::Xlu);
    }

    pub fn tex_mesh_xlu_resident(
        &mut self,
        model: Mat4,
        texture: TextureId,
        vertices: Arc<[TexMeshVertex]>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
    ) {
        self.push_tex_mesh(model, texture, vertices, Some(key), local_aabb, TexPass::Xlu);
    }

    /// 世界自发光纹理网格（Emissive pass：测深不写深、additive）。
    pub fn tex_mesh_emissive(
        &mut self,
        model: Mat4,
        texture: TextureId,
        vertices: Arc<[TexMeshVertex]>,
    ) {
        self.push_tex_mesh(model, texture, vertices, None, None, TexPass::Emissive);
    }

    pub fn tex_mesh_emissive_resident(
        &mut self,
        model: Mat4,
        texture: TextureId,
        vertices: Arc<[TexMeshVertex]>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
    ) {
        self.push_tex_mesh(model, texture, vertices, Some(key), local_aabb, TexPass::Emissive);
    }

    /// 不透明蒙皮网格（可选纹理；首切 palette ≤ [`MAX_SKIN_JOINTS`]）。
    pub fn skinned_mesh(
        &mut self,
        model: Mat4,
        vertices: Arc<[SkinnedVertex]>,
        joint_palette: Arc<[Mat4]>,
        texture: Option<TextureId>,
    ) {
        self.push_skinned_mesh(model, vertices, joint_palette, texture, None, None);
    }

    pub fn skinned_mesh_resident(
        &mut self,
        model: Mat4,
        vertices: Arc<[SkinnedVertex]>,
        joint_palette: Arc<[Mat4]>,
        texture: Option<TextureId>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
    ) {
        self.push_skinned_mesh(
            model,
            vertices,
            joint_palette,
            texture,
            Some(key),
            local_aabb,
        );
    }

    fn push_mesh(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        resident: Option<MeshResidentKey>,
        local_aabb: Option<Aabb3>,
        casts_shadow: bool,
    ) {
        if vertices.is_empty() {
            return;
        }
        self.meshes.push(MeshCmd {
            model,
            vertices,
            resident,
            local_aabb,
            casts_shadow,
        });
    }

    fn push_mesh_xlu(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        resident: Option<MeshResidentKey>,
        local_aabb: Option<Aabb3>,
    ) {
        if vertices.is_empty() {
            return;
        }
        self.meshes_xlu.push(MeshCmd {
            model,
            vertices,
            resident,
            local_aabb,
            casts_shadow: false,
        });
    }

    fn push_mesh_emissive(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        resident: Option<MeshResidentKey>,
        local_aabb: Option<Aabb3>,
    ) {
        if vertices.is_empty() {
            return;
        }
        self.meshes_emissive.push(MeshCmd {
            model,
            vertices,
            resident,
            local_aabb,
            casts_shadow: false,
        });
    }

    fn push_view_model_mesh(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        resident: Option<MeshResidentKey>,
        local_aabb: Option<Aabb3>,
    ) {
        if vertices.is_empty() {
            return;
        }
        self.view_model_meshes.push(MeshCmd {
            model,
            vertices,
            resident,
            local_aabb,
            casts_shadow: false,
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
            casts_shadow: false,
        });
    }

    fn push_sky_atmosphere(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        resident: Option<MeshResidentKey>,
        local_aabb: Option<Aabb3>,
    ) {
        if vertices.is_empty() {
            return;
        }
        self.sky_atmosphere_meshes.push(MeshCmd {
            model,
            vertices,
            resident,
            local_aabb,
            casts_shadow: false,
        });
    }

    fn push_sky_emissive(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        resident: Option<MeshResidentKey>,
        local_aabb: Option<Aabb3>,
    ) {
        if vertices.is_empty() {
            return;
        }
        self.sky_emissive_meshes.push(MeshCmd {
            model,
            vertices,
            resident,
            local_aabb,
            casts_shadow: false,
        });
    }

    fn push_tex_mesh(
        &mut self,
        model: Mat4,
        texture: TextureId,
        vertices: Arc<[TexMeshVertex]>,
        resident: Option<MeshResidentKey>,
        local_aabb: Option<Aabb3>,
        pass: TexPass,
    ) {
        if vertices.is_empty() {
            return;
        }
        let cmd = TexMeshCmd {
            model,
            texture,
            vertices,
            resident,
            local_aabb,
            // 不透明可投射；透明/自发光不进阴影深度。
            casts_shadow: matches!(pass, TexPass::Opaque),
        };
        match pass {
            TexPass::Opaque => self.tex_meshes.push(cmd),
            TexPass::Xlu => self.tex_meshes_xlu.push(cmd),
            TexPass::Emissive => self.tex_meshes_emissive.push(cmd),
        }
    }

    fn push_skinned_mesh(
        &mut self,
        model: Mat4,
        vertices: Arc<[SkinnedVertex]>,
        joint_palette: Arc<[Mat4]>,
        texture: Option<TextureId>,
        resident: Option<MeshResidentKey>,
        local_aabb: Option<Aabb3>,
    ) {
        if vertices.is_empty() || joint_palette.is_empty() {
            return;
        }
        let palette = if joint_palette.len() > MAX_SKIN_JOINTS {
            Arc::from(&joint_palette[..MAX_SKIN_JOINTS])
        } else {
            joint_palette
        };
        self.skinned_meshes.push(SkinnedMeshCmd {
            model,
            vertices,
            joint_palette: palette,
            texture,
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
        self.meshes_xlu
            .retain(|m| m.world_aabb().map(keep).unwrap_or(true));
        self.meshes_emissive
            .retain(|m| m.world_aabb().map(keep).unwrap_or(true));
        self.tex_meshes
            .retain(|m| m.world_aabb().map(keep).unwrap_or(true));
        self.tex_meshes_xlu
            .retain(|m| m.world_aabb().map(keep).unwrap_or(true));
        self.tex_meshes_emissive
            .retain(|m| m.world_aabb().map(keep).unwrap_or(true));
        self.skinned_meshes
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
        self.meshes_xlu
            .retain(|m| m.world_aabb().map(keep).unwrap_or(true));
        self.meshes_emissive
            .retain(|m| m.world_aabb().map(keep).unwrap_or(true));
        self.tex_meshes
            .retain(|m| m.world_aabb().map(keep).unwrap_or(true));
        self.tex_meshes_xlu
            .retain(|m| m.world_aabb().map(keep).unwrap_or(true));
        self.tex_meshes_emissive
            .retain(|m| m.world_aabb().map(keep).unwrap_or(true));
        self.skinned_meshes
            .retain(|m| m.world_aabb().map(keep).unwrap_or(true));
    }
}

#[derive(Debug, Clone, Copy)]
enum TexPass {
    Opaque,
    Xlu,
    Emissive,
}
