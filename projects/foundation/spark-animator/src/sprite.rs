//! 2D 精灵帧。只描述图集中的一格，不接触绘制列表。

use spark_types::Rect;

/// 精灵动画的一帧。
///
/// `region` 是归一化图集矩形。渲染侧用它选 UV，本 crate 不上传纹理。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteFrame {
    pub index: u32,
    pub region: Rect,
}

impl SpriteFrame {
    pub fn new(index: u32, region: Rect) -> Self {
        Self { index, region }
    }
}
