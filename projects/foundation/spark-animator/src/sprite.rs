//! 2D 精灵帧。只描述图集中的一格，不接触绘制列表。

use spark_types::Rect;

/// 精灵动画的一帧。
///
/// `region` 是归一化图集矩形。渲染侧用它选 UV，本 crate 不上传纹理。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteFrame {
    /// 图集内逻辑帧序号（调用方约定，引擎不解释）。
    pub index: u32,
    /// 归一化 UV 矩形（通常 `[0,1]²`）。
    pub region: Rect,
}

impl SpriteFrame {
    /// 构造一帧：序号 + 图集区域。
    pub fn new(index: u32, region: Rect) -> Self {
        Self { index, region }
    }
}
