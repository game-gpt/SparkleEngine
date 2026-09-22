//! 二维 RGB 光照网格。
//!
//! 只做列天光和四邻传播。格子是否挡光、光源颜色由调用方回调决定。

use std::collections::VecDeque;

/// 线性 RGB，不含 alpha。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightRgb {
    /// 红通道（线性光，通常 ≥ 0）。
    pub r: f32,
    /// 绿通道（线性光，通常 ≥ 0）。
    pub g: f32,
    /// 蓝通道（线性光，通常 ≥ 0）。
    pub b: f32,
}

impl LightRgb {
    /// 按通道构造。
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    /// 取三通道最大值，用作传播/裁剪的标量强度。
    pub fn intensity(self) -> f32 {
        self.r.max(self.g).max(self.b)
    }

    fn max_channel(self, other: Self, bump: f32) -> (Self, bool) {
        let mut out = self;
        let mut changed = false;
        if other.r > self.r + bump {
            out.r = other.r;
            changed = true;
        }
        if other.g > self.g + bump {
            out.g = other.g;
            changed = true;
        }
        if other.b > self.b + bump {
            out.b = other.b;
            changed = true;
        }
        (out, changed)
    }
}

/// 传播时开放格与挡光格的亮度下降量。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightFalloff {
    /// 开放格每步衰减（线性通道减法）。
    pub open: f32,
    /// 挡光格每步衰减（通常远大于 `open`）。
    pub blocked: f32,
    /// 新亮度必须高出这么多才继续入队。
    pub bump: f32,
    /// 低于此强度不再向外传。
    pub cutoff: f32,
}

impl Default for LightFalloff {
    fn default() -> Self {
        Self { open: 0.085, blocked: 0.38, bump: 0.008, cutoff: 0.06 }
    }
}

/// 原点在左上的矩形光照缓冲。坐标是世界格，不是缓冲下标。
#[derive(Debug, Clone)]
pub struct LightGrid2d {
    origin_x: i32,
    origin_y: i32,
    w: i32,
    h: i32,
    floor: LightRgb,
    cells: Vec<LightRgb>,
}

impl LightGrid2d {
    /// 以地板色填满 `w×h` 格；`origin_*` 为世界格坐标原点（左上）。
    pub fn filled(origin_x: i32, origin_y: i32, w: i32, h: i32, floor: LightRgb) -> Self {
        let w = w.max(0);
        let h = h.max(0);
        let n = (w as usize).saturating_mul(h as usize);
        Self { origin_x, origin_y, w, h, floor, cells: vec![floor; n] }
    }

    /// 缓冲宽度（格）。
    pub fn width(&self) -> i32 {
        self.w
    }

    /// 缓冲高度（格）。
    pub fn height(&self) -> i32 {
        self.h
    }

    /// 读取世界格亮度；越界返回地板色。
    pub fn get(&self, x: i32, y: i32) -> LightRgb {
        match self.index(x, y) {
            Some(i) => self.cells[i],
            None => self.floor,
        }
    }

    /// 只在更亮时写入。越界返回 false。
    pub fn set_max(&mut self, x: i32, y: i32, value: LightRgb) -> bool {
        let Some(i) = self.index(x, y)
        else {
            return false;
        };
        let (next, changed) = self.cells[i].max_channel(value, 0.0);
        if changed {
            self.cells[i] = next;
        }
        changed
    }

    /// 每列从 `origin_y` 向下写天光。`blocks` 为真时该列剩余格子保持地板亮度。
    pub fn paint_sky_columns(&mut self, sky: LightRgb, mut blocks: impl FnMut(i32, i32) -> bool) {
        for lx in 0..self.w {
            let x = self.origin_x + lx;
            for ly in 0..self.h {
                let y = self.origin_y + ly;
                if blocks(x, y) {
                    break;
                }
                let _ = self.set_max(x, y, sky);
            }
        }
    }

    /// 四邻传播。`blocked` 为真的格子用 [`LightFalloff::blocked`]。
    pub fn propagate(&mut self, falloff: LightFalloff, mut blocked: impl FnMut(i32, i32) -> bool) {
        let mut queue = VecDeque::new();
        for ly in 0..self.h {
            for lx in 0..self.w {
                let i = (ly * self.w + lx) as usize;
                if self.cells[i].intensity() > falloff.cutoff {
                    queue.push_back((lx, ly));
                }
            }
        }
        while let Some((lx, ly)) = queue.pop_front() {
            let i = (ly * self.w + lx) as usize;
            let cur = self.cells[i];
            if cur.intensity() <= falloff.cutoff {
                continue;
            }
            let x = self.origin_x + lx;
            let y = self.origin_y + ly;
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nlx = lx + dx;
                let nly = ly + dy;
                if nlx < 0 || nly < 0 || nlx >= self.w || nly >= self.h {
                    continue;
                }
                let nx = x + dx;
                let ny = y + dy;
                let fall = if blocked(nx, ny) { falloff.blocked } else { falloff.open };
                let next = LightRgb::new(cur.r - fall, cur.g - fall, cur.b - fall);
                let ni = (nly * self.w + nlx) as usize;
                let (raised, changed) = self.cells[ni].max_channel(next, falloff.bump);
                if changed {
                    self.cells[ni] = raised;
                    queue.push_back((nlx, nly));
                }
            }
        }
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        let lx = x - self.origin_x;
        let ly = y - self.origin_y;
        if lx < 0 || ly < 0 || lx >= self.w || ly >= self.h {
            return None;
        }
        Some((ly * self.w + lx) as usize)
    }
}
