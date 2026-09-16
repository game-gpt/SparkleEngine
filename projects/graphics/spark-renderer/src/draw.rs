use spark_core::{Color, Rect, Vec2};

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

/// 一帧绘制命令列表（屏幕像素坐标，原点左上）。
///
/// 与具体 GPU 后端无关；由 `spark-renderer-wgpu` 等实现提交。
#[derive(Debug)]
pub struct DrawList {
    pub clear: Color,
    pub quads: Vec<QuadCmd>,
    pub texts: Vec<TextCmd>,
}

impl DrawList {
    pub fn new(clear: Color) -> Self {
        Self {
            clear,
            quads: Vec::new(),
            texts: Vec::new(),
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
}
