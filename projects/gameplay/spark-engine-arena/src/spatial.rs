//! 均匀网格圆邻近查询（每帧重建）。

use std::collections::HashMap;

/// 圆心空间哈希；`cell_size` 应 ≥ 最大查询半径。
#[derive(Debug, Clone)]
pub struct CircleGrid {
    cell: f32,
    buckets: HashMap<(i32, i32), Vec<usize>>,
}

impl CircleGrid {
    /// `cell_size` 过小会浪费桶，过大漏检；通常取最大实体直径。
    pub fn new(cell_size: f32) -> Self {
        Self { cell: cell_size.max(1.0), buckets: HashMap::new() }
    }

    /// 清空桶（保留容量）。
    pub fn clear(&mut self) {
        for v in self.buckets.values_mut() {
            v.clear();
        }
    }

    fn key(&self, x: f32, y: f32) -> (i32, i32) {
        ((x / self.cell).floor() as i32, (y / self.cell).floor() as i32)
    }

    /// 插入实体索引与圆心。
    pub fn insert(&mut self, index: usize, x: f32, y: f32) {
        let k = self.key(x, y);
        self.buckets.entry(k).or_default().push(index);
    }

    /// 查询与 `(x,y)` 距离 ≤ `radius` 的候选索引（含粗筛，需窄相确认）。
    pub fn query(&self, x: f32, y: f32, radius: f32) -> Vec<usize> {
        let r_cells = (radius / self.cell).ceil() as i32 + 1;
        let cx = (x / self.cell).floor() as i32;
        let cy = (y / self.cell).floor() as i32;
        let mut out = Vec::new();
        for dy in -r_cells..=r_cells {
            for dx in -r_cells..=r_cells {
                if let Some(list) = self.buckets.get(&(cx + dx, cy + dy)) {
                    out.extend_from_slice(list);
                }
            }
        }
        out
    }
}

/// 圆-圆窄相。
pub fn circles_hit(ax: f32, ay: f32, ar: f32, bx: f32, by: f32, br: f32) -> bool {
    let dx = ax - bx;
    let dy = ay - by;
    let rr = ar + br;
    dx * dx + dy * dy <= rr * rr
}
