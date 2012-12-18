//! 简易迷雾：未探索 / 可见（无战争迷雾渐变美术）。

use spark_core::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FogCell {
    Hidden,
    Explored,
    Visible,
}

#[derive(Debug, Clone)]
pub struct FogGrid {
    pub width: u32,
    pub height: u32,
    pub cell_size: f32,
    cells: Vec<FogCell>,
}

impl FogGrid {
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

    pub fn cell(&self, cx: u32, cy: u32) -> FogCell {
        self.idx(cx, cy).map(|i| self.cells[i]).unwrap_or(FogCell::Hidden)
    }

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
