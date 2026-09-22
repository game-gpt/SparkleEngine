//! 宽相：均匀网格。
//!
//! 只做潜在重叠对过滤；真正相交判定在窄相（`spark-geometry`）。

use std::collections::HashMap;

use spark_types::Rect;

use crate::body::BodyId;

/// 宽相接口：清空 → 插入包围盒 → 查询候选对。
///
/// 不变式：`query_pairs` 返回的对无序、无重复（规范化为较小 `BodyId` 在前）。
pub trait Broadphase {
    /// 丢弃上一帧全部格子内容。
    fn clear(&mut self);
    /// 将刚体按其 AABB 登记进覆盖到的格子；同一 ID 可跨多格重复出现。
    fn insert(&mut self, id: BodyId, bounds: Rect);
    /// 同格共现的无序唯一对；不保证世界空间真正相交。
    fn query_pairs(&self) -> Vec<(BodyId, BodyId)>;
}

/// 固定 cell 尺寸的均匀网格宽相。
///
/// `cell` 在构造时钳到至少 `1.0`（世界单位），避免除零与过密哈希。
#[derive(Debug, Clone)]
pub struct UniformGrid {
    cell: f32,
    cells: HashMap<(i32, i32), Vec<BodyId>>,
}

impl UniformGrid {
    /// 以世界单位边长创建网格；`cell_size < 1` 时抬到 `1.0`。
    pub fn new(cell_size: f32) -> Self {
        Self { cell: cell_size.max(1.0), cells: HashMap::new() }
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
