//! 球拍。

/// 矩形球拍：位置与尺寸为逻辑像素，`speed` 为竖直移动速度（像素/秒）。
#[derive(Debug, Clone)]
pub struct Paddle {
    /// 左上角 X（像素）。
    pub x: f32,
    /// 左上角 Y（像素）；向下为正。
    pub y: f32,
    /// 宽度（像素）。默认构造为 14。
    pub w: f32,
    /// 高度（像素）。默认构造为 80。
    pub h: f32,
    /// 竖直移动速度（像素/秒）。默认 420。
    pub speed: f32,
    /// 所属侧，仅作语义标记，不参与物理。
    pub side: PaddleSide,
}

/// 球拍所在半场。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddleSide {
    /// 左侧（开局 X≈24）。
    Left,
    /// 右侧（开局贴右边界内侧）。
    Right,
}

impl Paddle {
    /// 左侧默认拍：X=24，竖直居中于 `court_h`，高 80、速 420。
    pub fn left(court_h: f32) -> Self {
        Self { x: 24.0, y: court_h * 0.5 - 40.0, w: 14.0, h: 80.0, speed: 420.0, side: PaddleSide::Left }
    }

    /// 右侧默认拍：右缘距球场右边界 24 像素，竖直居中，尺寸同左拍。
    pub fn right(court_w: f32, court_h: f32) -> Self {
        Self { x: court_w - 38.0, y: court_h * 0.5 - 40.0, w: 14.0, h: 80.0, speed: 420.0, side: PaddleSide::Right }
    }

    /// 将 `y` 钳在 `[0, court_h - h]`，避免拍体越出上下边界。
    pub fn clamp_y(&mut self, court_h: f32) {
        self.y = self.y.clamp(0.0, (court_h - self.h).max(0.0));
    }
}
