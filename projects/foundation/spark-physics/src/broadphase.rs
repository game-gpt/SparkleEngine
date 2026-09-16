//! 宽相：均匀网格。

use std::collections::HashMap;

use spark_core::Rect;

use crate::body::BodyId;

pub trait Broadphase {
    fn clear(&mut self);
    fn insert(&mut self, id: BodyId, bounds: Rect);
    fn query_pairs(&self) -> Vec<(BodyId, BodyId)>;
}

/// 固定 cell 尺寸的均匀网格宽相。
#[derive(Debug, Clone)]
pub struct UniformGrid {
    cell: f32,
    cells: HashMap<(i32, i32), Vec<BodyId>>,
}

impl UniformGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell: cell_size.max(1.0),
            cells: HashMap::new(),
        }
    }
}

impl Broadphase for UniformGrid {
    fn clear(&mut self) {
        self.cells.clear();
    }

    fn insert(&mut self, id: BodyId, bounds: Rect) {
        let c0x = (bounds.x / self.cell).floor() as i32;
        let c0y = (bounds.y / self.cell).floor() as i32;
        let c1x = ((bounds.x + bounds.w) / self.cell).floor() as i32;
        let c1y = ((bounds.y + bounds.h) / self.cell).floor() as i32;
        for cy in c0y..=c1y {
            for cx in c0x..=c1x {
                self.cells.entry((cx, cy)).or_default().push(id);
            }
        }
    }

    fn query_pairs(&self) -> Vec<(BodyId, BodyId)> {
        let mut pairs = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for ids in self.cells.values() {
            for i in 0..ids.len() {
                for j in (i + 1)..ids.len() {
                    let a = ids[i];
                    let b = ids[j];
                    let key = if a.0 < b.0 { (a, b) } else { (b, a) };
                    if seen.insert(key) {
                        pairs.push(key);
                    }
                }
            }
        }
        pairs
    }
}
