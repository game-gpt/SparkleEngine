//! 屏幕空间 HUD 绘制目标：供装饰层 / 叠绘写入 [`UiRenderBatch`]。

use spark_types::{Color, Rect};

use crate::texture::TextureId;
use crate::ui_batch::UiRenderBatch;

/// 屏幕像素 HUD 画布（无世界相机变换）。
///
/// Widget paint 与 TitleChrome 等装饰应写入本接口的实现（通常是 [`UiRenderBatch`]），
/// 而不是往 [`crate::DrawList`] 的 `hud_*` 里塞命令。
pub trait HudCanvas {
    /// 填充轴对齐矩形。
    fn fill_rect(&mut self, rect: Rect, color: Color);
    /// 排队一行文字。
    fn text(&mut self, x: f32, y: f32, size: f32, color: Color, text: impl Into<String>);
    /// 纹理四边形（轴对齐）。
    fn tex_rect(&mut self, texture: TextureId, dest: Rect, uv: Rect, color: Color);
}

impl HudCanvas for UiRenderBatch {
    fn fill_rect(&mut self, rect: Rect, color: Color) {
        Self::fill_rect(self, rect, color);
    }

    fn text(&mut self, x: f32, y: f32, size: f32, color: Color, text: impl Into<String>) {
        Self::text(self, x, y, size, color, text);
    }

    fn tex_rect(&mut self, texture: TextureId, dest: Rect, uv: Rect, color: Color) {
        Self::tex_rect(self, texture, dest, uv, color);
    }
}
