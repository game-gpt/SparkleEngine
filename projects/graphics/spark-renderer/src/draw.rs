use spark_texture::TextureUpload;
use spark_types::{Color, Rect, SparkError, Vec2};

use crate::{
    camera2d::Camera2d,
    texture::{TextureId, alloc_texture_id},
};

/// 纯色轴对齐四边形命令（屏幕像素，原点左上）。
///
/// 世界层经 [`DrawList`] 相机变换后再入批；HUD 层坐标原样保留。
#[derive(Debug, Clone)]
pub struct QuadCmd {
    /// 目标矩形（像素）。世界层已乘相机 `zoom` 并相对 `origin` 偏移。
    pub rect: Rect,
    /// 填充色（含 alpha；后端按预乘/混合约定提交）。
    pub color: Color,
}

/// 屏幕空间文字命令（基线近似在 `pos`，字高由 `size` 给出）。
///
/// 始终走 HUD 文字批次，不受世界相机变换。
#[derive(Debug, Clone)]
pub struct TextCmd {
    /// 文字起点（屏幕像素，左上坐标系）。
    pub pos: Vec2,
    /// 近似字高（像素）；具体字形度量由后端字体管线解释。
    pub size: f32,
    /// 文字颜色（含 alpha）。
    pub color: Color,
    /// UTF-8 文本内容。
    pub text: String,
}

/// 屏幕空间纹理四边形（归一化 UV，颜色相乘）。
#[derive(Debug, Clone)]
pub struct TexQuadCmd {
    /// 采样用纹理句柄（须已通过 `create_texture*` / `queue_texture_upload` 入队）。
    pub texture: TextureId,
    /// 目标矩形（像素）。世界层已应用相机变换。
    pub dest: Rect,
    /// 源 UV 矩形，归一化到 `[0,1]`（相对整张纹理）。
    pub uv: Rect,
    /// 与纹素相乘的顶点色（含 alpha）。
    pub color: Color,
    /// 绕旋转枢轴逆时针旋转（弧度）。`0` 为轴对齐。
    pub angle_rad: f32,
    /// 相对 `dest` 左上角的旋转枢轴 X（像素；世界层已乘 `zoom`）。
    pub pivot_x: f32,
    /// 相对 `dest` 左上角的旋转枢轴 Y（像素；世界层已乘 `zoom`）。
    pub pivot_y: f32,
}

/// 2D 绘制层：世界在下，HUD 在上（同层内纹理压在纯色之上）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrawLayer2d {
    /// 世界层：后续 `fill_rect` / `tex_rect*` 吃 [`Camera2d`] 变换。
    #[default]
    World,
    /// HUD 层：屏幕像素坐标，叠在世界之上，不受相机影响。
    Hud,
}

/// 一帧绘制命令列表（屏幕像素坐标，原点左上）。
///
/// 与具体 GPU 后端无关；由 `spark-renderer-wgpu` 等实现提交。
/// 提交顺序：`world` 纯色 → `world` 纹理 → `hud` 纯色 → `hud` 纹理 → 文字。
/// [`push_clip`] 在推入命令时做 CPU 裁剪，GPU scissor 可后接。
#[derive(Debug)]
pub struct DrawList {
    /// 本帧清屏色（RGBA 线性浮点语义由后端解释）。
    pub clear: Color,
    /// 世界层纯色四边形批次。
    pub quads: Vec<QuadCmd>,
    /// 世界层纹理四边形批次（同层内排在纯色之后）。
    pub tex_quads: Vec<TexQuadCmd>,
    /// HUD 层纯色四边形批次。
    pub hud_quads: Vec<QuadCmd>,
    /// HUD 层纹理四边形批次。
    pub hud_tex_quads: Vec<TexQuadCmd>,
    /// 文字批次（始终屏幕空间，最后提交）。
    pub texts: Vec<TextCmd>,
    /// 本帧待上传纹理：`(句柄, 像素/压缩数据)`，由后端在 draw 前提交 GPU。
    pub texture_uploads: Vec<(TextureId, TextureUpload)>,
    layer: DrawLayer2d,
    /// 只变换世界层。默认原点与缩放为恒等，旧调用坐标不变。
    camera: Camera2d,
    clip_stack: Vec<Rect>,
}

impl DrawList {
    /// 新建空列表，指定清屏色；默认世界层、恒等相机、无裁剪栈。
    pub fn new(clear: Color) -> Self {
        Self {
            clear,
            quads: Vec::new(),
            tex_quads: Vec::new(),
            hud_quads: Vec::new(),
            hud_tex_quads: Vec::new(),
            texts: Vec::new(),
            texture_uploads: Vec::new(),
            layer: DrawLayer2d::World,
            camera: Camera2d::default(),
            clip_stack: Vec::new(),
        }
    }

    /// 设置世界层相机（`origin` 为视口左上世界坐标，`zoom` 为世界→像素缩放）。
    pub fn set_camera(&mut self, camera: Camera2d) {
        self.camera = camera;
    }

    /// 当前世界层相机副本。
    pub fn camera(&self) -> Camera2d {
        self.camera
    }

    /// 压入裁剪矩形（像素）。与栈顶求交；后续绘制在推入前做 CPU 相交裁剪。
    pub fn push_clip(&mut self, rect: Rect) {
        let next = match self.clip_stack.last() {
            Some(prev) => prev.intersect(rect),
            None => rect,
        };
        self.clip_stack.push(next);
    }

    /// 弹出最近一次 [`push_clip`]。栈空时无操作。
    pub fn pop_clip(&mut self) {
        let _ = self.clip_stack.pop();
    }

    /// 当前生效裁剪区（栈顶相交结果）；无裁剪时为 `None`。
    pub fn clip_rect(&self) -> Option<Rect> {
        self.clip_stack.last().copied()
    }

    fn clip_against_stack(&self, rect: Rect) -> Option<Rect> {
        let clipped = match self.clip_stack.last() {
            Some(clip) => rect.intersect(*clip),
            None => rect,
        };
        if clipped.is_empty() { None } else { Some(clipped) }
    }

    fn map_world(&self, rect: Rect) -> Rect {
        if self.layer != DrawLayer2d::World {
            return rect;
        }
        let z = if self.camera.zoom.is_finite() && self.camera.zoom > 1.0e-6 { self.camera.zoom } else { 1.0 };
        Rect::new((rect.x - self.camera.origin.x) * z, (rect.y - self.camera.origin.y) * z, rect.w * z, rect.h * z)
    }

    /// 切换到世界层：后续图元经相机变换并写入 `quads` / `tex_quads`。
    pub fn begin_world(&mut self) {
        self.layer = DrawLayer2d::World;
    }

    /// 切换到 HUD 层：后续图元用屏幕像素，写入 `hud_*` 批次。
    pub fn begin_hud(&mut self) {
        self.layer = DrawLayer2d::Hud;
    }

    /// 填充轴对齐矩形。世界层坐标为世界单位；HUD 为屏幕像素。空裁剪相交则丢弃。
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let Some(rect) = self.clip_against_stack(rect)
        else {
            return;
        };
        let q = QuadCmd { rect: self.map_world(rect), color };
        match self.layer {
            DrawLayer2d::World => self.quads.push(q),
            DrawLayer2d::Hud => self.hud_quads.push(q),
        }
    }

    /// 排队一行文字（屏幕空间）。基线点落在裁剪区外时整行跳过。
    pub fn text(&mut self, x: f32, y: f32, size: f32, color: Color, text: impl Into<String>) {
        if let Some(clip) = self.clip_stack.last() {
            // 粗裁：基线点不在裁剪区则跳过整行。
            if !clip.contains(Vec2::new(x, y + size * 0.5)) {
                return;
            }
        }
        self.texts.push(TextCmd { pos: Vec2::new(x, y), size, color, text: text.into() });
    }

    /// 分配稳定纹理 ID 并排队上传。请缓存返回的 ID。
    pub fn create_texture(&mut self, width: u32, height: u32, rgba: Vec<u8>) -> Result<TextureId, SparkError> {
        let upload = TextureUpload::rgba8_srgb(width, height, rgba)?;
        Ok(self.create_texture_upload(upload))
    }

    /// 排队任意 [`TextureUpload`]（压缩 / 多 mip 等）。
    pub fn create_texture_upload(&mut self, upload: TextureUpload) -> TextureId {
        let id = alloc_texture_id();
        self.texture_uploads.push((id, upload));
        id
    }

    /// 用已有 ID 重新上传像素。
    pub fn update_texture(&mut self, id: TextureId, width: u32, height: u32, rgba: Vec<u8>) -> Result<(), SparkError> {
        let upload = TextureUpload::rgba8_srgb(width, height, rgba)?;
        self.queue_texture_upload(id, upload);
        Ok(())
    }

    /// 用已有 ID 排队 [`TextureUpload`]。
    pub fn queue_texture_upload(&mut self, id: TextureId, upload: TextureUpload) {
        self.texture_uploads.push((id, upload));
    }

    /// 绘制纹理四边形。`uv` 为归一化 [0,1] 源矩形。
    pub fn tex_rect(&mut self, texture: TextureId, dest: Rect, uv: Rect, color: Color) {
        let px = dest.w * 0.5;
        let py = dest.h * 0.5;
        self.tex_rect_rot(texture, dest, uv, color, 0.0, px, py);
    }

    /// 绘制纹理四边形，绕相对 `dest` 左上角的枢轴旋转 `angle_rad` 弧度。
    pub fn tex_rect_rot(&mut self, texture: TextureId, dest: Rect, uv: Rect, color: Color, angle_rad: f32, pivot_x: f32, pivot_y: f32) {
        let Some(dest) = self.clip_against_stack(dest)
        else {
            return;
        };
        let dest = self.map_world(dest);
        let z = if self.layer == DrawLayer2d::World {
            let zoom = self.camera.zoom;
            if zoom.is_finite() && zoom > 1.0e-6 { zoom } else { 1.0 }
        }
        else {
            1.0
        };
        let q = TexQuadCmd { texture, dest, uv, color, angle_rad, pivot_x: pivot_x * z, pivot_y: pivot_y * z };
        match self.layer {
            DrawLayer2d::World => self.tex_quads.push(q),
            DrawLayer2d::Hud => self.hud_tex_quads.push(q),
        }
    }
}
