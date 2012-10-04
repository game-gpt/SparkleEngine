use spark_core::{Color, Rect, SparkError, Vec2};

use crate::camera2d::Camera2d;
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
    /// 绕旋转枢轴逆时针旋转（弧度）。`0` 为轴对齐。
    pub angle_rad: f32,
    /// 相对 `dest` 左上角的旋转枢轴（像素）。
    pub pivot_x: f32,
    pub pivot_y: f32,
}

/// 2D 绘制层：世界在下，HUD 在上（同层内纹理压在纯色之上）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrawLayer2d {
    #[default]
    World,
    Hud,
}

/// 一帧绘制命令列表（屏幕像素坐标，原点左上）。
///
/// 与具体 GPU 后端无关；由 `spark-renderer-wgpu` 等实现提交。
/// 提交顺序：`world` 纯色 → `world` 纹理 → `hud` 纯色 → `hud` 纹理 → 文字。
#[derive(Debug)]
pub struct DrawList {
    pub clear: Color,
    pub quads: Vec<QuadCmd>,
    pub tex_quads: Vec<TexQuadCmd>,
    pub hud_quads: Vec<QuadCmd>,
    pub hud_tex_quads: Vec<TexQuadCmd>,
    pub texts: Vec<TextCmd>,
    pub texture_uploads: Vec<(TextureId, RgbaImage)>,
    layer: DrawLayer2d,
    /// 只变换世界层。默认原点与缩放为恒等，旧调用坐标不变。
    camera: Camera2d,
}

impl DrawList {
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
        }
    }

    pub fn set_camera(&mut self, camera: Camera2d) {
        self.camera = camera;
    }

    pub fn camera(&self) -> Camera2d {
        self.camera
    }

    fn map_world(&self, rect: Rect) -> Rect {
        if self.layer != DrawLayer2d::World {
            return rect;
        }
        let z = if self.camera.zoom.is_finite() && self.camera.zoom > 1.0e-6 {
            self.camera.zoom
        } else {
            1.0
        };
        Rect::new(
            (rect.x - self.camera.origin.x) * z,
            (rect.y - self.camera.origin.y) * z,
            rect.w * z,
            rect.h * z,
        )
    }

    pub fn begin_world(&mut self) {
        self.layer = DrawLayer2d::World;
    }

    pub fn begin_hud(&mut self) {
        self.layer = DrawLayer2d::Hud;
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let q = QuadCmd {
            rect: self.map_world(rect),
            color,
        };
        match self.layer {
            DrawLayer2d::World => self.quads.push(q),
            DrawLayer2d::Hud => self.hud_quads.push(q),
        }
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
        let px = dest.w * 0.5;
        let py = dest.h * 0.5;
        self.tex_rect_rot(texture, dest, uv, color, 0.0, px, py);
    }

    /// 绘制纹理四边形，绕相对 `dest` 左上角的枢轴旋转 `angle_rad` 弧度。
    pub fn tex_rect_rot(
        &mut self,
        texture: TextureId,
        dest: Rect,
        uv: Rect,
        color: Color,
        angle_rad: f32,
        pivot_x: f32,
        pivot_y: f32,
    ) {
        let dest = self.map_world(dest);
        let z = if self.layer == DrawLayer2d::World {
            let zoom = self.camera.zoom;
            if zoom.is_finite() && zoom > 1.0e-6 {
                zoom
            } else {
                1.0
            }
        } else {
            1.0
        };
        let q = TexQuadCmd {
            texture,
            dest,
            uv,
            color,
            angle_rad,
            pivot_x: pivot_x * z,
            pivot_y: pivot_y * z,
        };
        match self.layer {
            DrawLayer2d::World => self.tex_quads.push(q),
            DrawLayer2d::Hud => self.hud_tex_quads.push(q),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_core::Color;

    #[test]
    fn world_quads_follow_camera_and_hud_does_not() {
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        draw.set_camera(Camera2d::new(Vec2::new(8.0, 0.0), 2.0));
        draw.fill_rect(Rect::new(10.0, 0.0, 4.0, 2.0), Color::rgb(1.0, 0.0, 0.0));
        draw.begin_hud();
        draw.fill_rect(Rect::new(10.0, 0.0, 4.0, 2.0), Color::rgb(0.0, 1.0, 0.0));
        assert!((draw.quads[0].rect.x - 4.0).abs() < 1e-5);
        assert!((draw.quads[0].rect.w - 8.0).abs() < 1e-5);
        assert!((draw.hud_quads[0].rect.x - 10.0).abs() < 1e-5);
        assert!((draw.hud_quads[0].rect.w - 4.0).abs() < 1e-5);
    }
}
