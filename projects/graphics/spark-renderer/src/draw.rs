use spark_core::{Color, Rect, SparkError, Vec2};

use crate::texture::{alloc_texture_id, RgbaImage, TextureId};

#[derive(Debug, Clone)]
pub struct QuadCmd {
    pub rect: Rect,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct TextCmd {
    pub pos: Vec2,
    pub size: f32,
    pub color: Color,
    pub text: String,
}

/// 屏幕空间纹理四边形（归一化 UV，颜色相乘）。
#[derive(Debug, Clone)]
pub struct TexQuadCmd {
    pub texture: TextureId,
    pub dest: Rect,
    pub uv: Rect,
    pub color: Color,
}

/// 一帧绘制命令列表（屏幕像素坐标，原点左上）。
///
/// 与具体 GPU 后端无关；由 `spark-renderer-wgpu` 等实现提交。
#[derive(Debug)]
pub struct DrawList {
    pub clear: Color,
    pub quads: Vec<QuadCmd>,
    pub texts: Vec<TextCmd>,
    pub tex_quads: Vec<TexQuadCmd>,
    pub texture_uploads: Vec<(TextureId, RgbaImage)>,
}

impl DrawList {
    pub fn new(clear: Color) -> Self {
        Self {
            clear,
            quads: Vec::new(),
            texts: Vec::new(),
            tex_quads: Vec::new(),
            texture_uploads: Vec::new(),
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.quads.push(QuadCmd { rect, color });
    }

    pub fn text(&mut self, x: f32, y: f32, size: f32, color: Color, text: impl Into<String>) {
        self.texts.push(TextCmd {
            pos: Vec2::new(x, y),
            size,
            color,
            text: text.into(),
        });
    }

    /// 分配稳定纹理 ID 并排队上传。请缓存返回的 ID。
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

    /// 用已有 ID 重新上传像素。
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

    /// 绘制纹理四边形。`uv` 为归一化 [0,1] 源矩形。
    pub fn tex_rect(&mut self, texture: TextureId, dest: Rect, uv: Rect, color: Color) {
        self.tex_quads.push(TexQuadCmd {
            texture,
            dest,
            uv,
            color,
        });
    }
}
