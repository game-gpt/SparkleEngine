//! 简易迷雾：未探索 / 可见（无战争迷雾渐变美术）。
//!
//! 每帧应先 [`FogGrid::clear_visible`]（Visible → Explored），再对友军调用 [`FogGrid::reveal`]。

use spark_types::Vec2;

/// 单格迷雾状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FogCell {
    /// 从未揭示。
    Hidden,
    /// 曾揭示，当前不可见。
    Explored,
    /// 本帧可见。
    Visible,
}

/// 规则网格迷雾。
#[derive(Debug, Clone)]
pub struct FogGrid {
    /// 横向格子数。
    pub width: u32,
    /// 纵向格子数。
    pub height: u32,
    /// 格子边长（世界单位）；构造时至少 `1e-3`。
    pub cell_size: f32,
    cells: Vec<FogCell>,
}

impl FogGrid {
    /// 全图初始化为 [`FogCell::Hidden`]。
    pub fn new(width: u32, height: u32, cell_size: f32) -> Self {
        let n = (width as usize).saturating_mul(height as usize);
        Self { width, height, cell_size: cell_size.max(1e-3), cells: vec![FogCell::Hidden; n] }
    }

    fn idx(&self, cx: u32, cy: u32) -> Option<usize> {
        if cx >= self.width || cy >= self.height {
            return None;
        }
        Some((cy * self.width + cx) as usize)
    }

    /// 查询格子；越界视为 [`FogCell::Hidden`]。
    pub fn cell(&self, cx: u32, cy: u32) -> FogCell {
        self.idx(cx, cy).map(|i| self.cells[i]).unwrap_or(FogCell::Hidden)
    }

    /// 是否为本帧可见格。
    pub fn is_visible(&self, cx: u32, cy: u32) -> bool {
        self.cell(cx, cy) == FogCell::Visible
    }

    /// 每帧先把 Visible 降为 Explored。
    pub fn clear_visible(&mut self) {
        for c in &mut self.cells {
            if *c == FogCell::Visible {
                *c = FogCell::Explored;
            }
        }
    }

    /// 以世界点为圆心、`radius` 为半径将圆内格子标为 Visible。
    pub fn reveal(&mut self, world: Vec2, radius: f32) {
        let cs = self.cell_size;
        let r = radius.max(0.0);
        let cx0 = ((world.x - r) / cs).floor() as i32;
        let cy0 = ((world.y - r) / cs).floor() as i32;
        let cx1 = ((world.x + r) / cs).ceil() as i32;
        let cy1 = ((world.y + r) / cs).ceil() as i32;
        let r2 = r * r;
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                if cx < 0 || cy < 0 {
                    continue;
                }
                let (cx, cy) = (cx as u32, cy as u32);
                let Some(i) = self.idx(cx, cy)
                else {
                    continue;
                };
                let wx = (cx as f32 + 0.5) * cs;
                let wy = (cy as f32 + 0.5) * cs;
                let dx = wx - world.x;
                let dy = wy - world.y;
                if dx * dx + dy * dy <= r2 {
                    self.cells[i] = FogCell::Visible;
                }
            }
        }
    }
}
