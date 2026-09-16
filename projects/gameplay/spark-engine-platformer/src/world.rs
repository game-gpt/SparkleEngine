//! 固体碰撞世界。

use spark_core::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidKind {
    /// 四面阻挡。
    Solid,
    /// 仅从上落下时站立（单向台）。
    OneWay,
}

#[derive(Debug, Clone, Copy)]
pub struct SolidRect {
    pub rect: Rect,
    pub kind: SolidKind,
}

#[derive(Debug, Default, Clone)]
pub struct TileWorld {
    solids: Vec<SolidRect>,
}

impl TileWorld {
    pub fn push(&mut self, s: SolidRect) {
        self.solids.push(s);
    }

    pub fn clear(&mut self) {
        self.solids.clear();
    }

    pub fn solids(&self) -> &[SolidRect] {
        &self.solids
    }
}
