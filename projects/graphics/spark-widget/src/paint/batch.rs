//! UI 专用绘制批次：把 Widget paint 与游戏世界 [`DrawList`] 解耦。

use spark_renderer::{DrawList, QuadCmd, TexQuadCmd, TextCmd, TextureId};
use spark_types::{Color, Rect, Vec2};

/// Widget 树产出的 HUD 绘制命令（纯色 / 纹理 / 文字）。
///
/// 当前经 [`Self::flush_hud`] 写入 [`DrawList`] 的 HUD 层；后续可直连 wgpu 而不经世界 DrawList。
#[derive(Debug, Default)]
pub struct UiRenderBatch {
    /// 轴对齐纯色四边形。
    pub quads: Vec<QuadCmd>,
    /// 纹理四边形。
    pub tex_quads: Vec<TexQuadCmd>,
    /// 屏幕空间文字。
    pub texts: Vec<TextCmd>,
    clip_stack: Vec<Rect>,
}

impl UiRenderBatch {
    /// 空批次。
    pub fn new() -> Self {
        Self::default()
    }

    /// 清空命令与裁剪栈，保留容量。
    pub fn clear(&mut self) {
        self.quads.clear();
        self.tex_quads.clear();
        self.texts.clear();
        self.clip_stack.clear();
    }

    /// 压入裁剪矩形（与栈顶求交）。
    pub fn push_clip(&mut self, rect: Rect) {
        let next = match self.clip_stack.last() {
            Some(prev) => prev.intersect(rect),
            None => rect,
        };
        self.clip_stack.push(next);
    }

    /// 弹出裁剪；栈空时无操作。
    pub fn pop_clip(&mut self) {
        let _ = self.clip_stack.pop();
    }

    /// 当前裁剪区。
    pub fn clip_rect(&self) -> Option<Rect> {
        self.clip_stack.last().copied()
    }

    fn clip_against_stack(&self, rect: Rect) -> Option<Rect> {
        let clipped = match self.clip_stack.last() {
            Some(clip) => rect.intersect(*clip),
            None => rect,
        };
        if clipped.is_empty() {
            None
        } else {
            Some(clipped)
        }
    }

    /// 填充轴对齐矩形（已裁剪）。
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let Some(rect) = self.clip_against_stack(rect) else {
            return;
        };
        self.quads.push(QuadCmd { rect, color });
    }

    /// 排队一行文字；基线中点落在裁剪区外则整行跳过。
    pub fn text(&mut self, x: f32, y: f32, size: f32, color: Color, text: impl Into<String>) {
        if let Some(clip) = self.clip_stack.last() {
            if !clip.contains(Vec2::new(x, y + size * 0.5)) {
                return;
            }
        }
        self.texts.push(TextCmd {
            pos: Vec2::new(x, y),
            size,
            color,
            text: text.into(),
        });
    }

    /// 先按 `offsets` 画描边，再画正文（同一文案）。
    pub fn text_outlined(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
        outline: Color,
        offsets: &[(f32, f32)],
        text: &str,
    ) {
        for &(ox, oy) in offsets {
            self.text(x + ox, y + oy, size, outline, text);
        }
        self.text(x, y, size, color, text);
    }

    /// 纹理四边形（轴对齐）。
    pub fn tex_rect(&mut self, texture: TextureId, dest: Rect, uv: Rect, color: Color) {
        let Some(dest) = self.clip_against_stack(dest) else {
            return;
        };
        let px = dest.w * 0.5;
        let py = dest.h * 0.5;
        self.tex_quads.push(TexQuadCmd {
            texture,
            dest,
            uv,
            color,
            angle_rad: 0.0,
            pivot_x: px,
            pivot_y: py,
        });
    }

    /// 本批命令数（调试 / 测试）。
    pub fn command_count(&self) -> usize {
        self.quads.len() + self.tex_quads.len() + self.texts.len()
    }

    /// 把本批刷入 [`DrawList`] 的 HUD 层（不改动世界层命令）。
    pub fn flush_hud(&mut self, draw: &mut DrawList) {
        draw.begin_hud();
        draw.hud_quads.append(&mut self.quads);
        draw.hud_tex_quads.append(&mut self.tex_quads);
        draw.texts.append(&mut self.texts);
        self.clip_stack.clear();
    }
}
