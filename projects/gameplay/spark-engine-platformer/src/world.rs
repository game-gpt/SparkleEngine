//! 固体碰撞世界。
//!
//! 仅轴对齐矩形列表，无瓦片网格权威；游戏仓负责把关卡数据压成 [`SolidRect`]。

use spark_types::Rect;

/// 固体碰撞语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidKind {
    /// 四面阻挡。
    Solid,
    /// 仅从上落下时站立（单向台）。
    OneWay,
}

/// 世界中的一块固体。
#[derive(Debug, Clone, Copy)]
pub struct SolidRect {
    /// 轴对齐矩形（世界坐标）。
    pub rect: Rect,
    /// 碰撞语义。
    pub kind: SolidKind,
}

/// 固体集合（顺序即解析顺序）。
#[derive(Debug, Default, Clone)]
pub struct TileWorld {
    solids: Vec<SolidRect>,
}

impl TileWorld {
    /// 追加一块固体。
    pub fn push(&mut self, s: SolidRect) {
        self.solids.push(s);
    }

    /// 清空全部固体。
    pub fn clear(&mut self) {
        self.solids.clear();
    }

    /// 只读固体切片。
    pub fn solids(&self) -> &[SolidRect] {
        &self.solids
    }
}
