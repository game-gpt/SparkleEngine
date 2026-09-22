//! 布局规格与计算结果。

use spark_types::Rect;

use super::{Align, FlexDirection, Justify, Size};

/// 节点选用的布局算法种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// 子项叠放，后声明的画在上层。
    Stack,
    /// Flex 流式排布。
    Flex,
    /// 固定列数网格。
    Grid,
    /// 子项铺满父内容区（浮层叠放）。
    Overlay,
    /// 相对父锚点定位。
    Anchor,
    /// 绝对坐标（配合 `offset_x` / `offset_y`）。
    Absolute,
}

impl Default for Layout {
    fn default() -> Self {
        Self::Flex
    }
}

/// 节点声明的布局意图（引擎据此 measure / place）。
#[derive(Debug, Clone)]
pub struct LayoutSpec {
    /// 布局算法种类。
    pub kind: Layout,
    /// Flex 主轴方向（仅 `Layout::Flex` 有意义）。
    pub direction: FlexDirection,
    /// 期望宽度。
    pub width: Size,
    /// 期望高度。
    pub height: Size,
    /// 最小宽度（像素，可选）。
    pub min_width: Option<f32>,
    /// 最小高度（像素，可选）。
    pub min_height: Option<f32>,
    /// 最大宽度（像素，可选）。
    pub max_width: Option<f32>,
    /// 最大高度（像素，可选）。
    pub max_height: Option<f32>,
    /// 内边距。
    pub padding: Insets,
    /// 外边距。
    pub margin: Insets,
    /// 子项间隙（Flex / Grid）。
    pub gap: f32,
    /// 交叉轴对齐。
    pub align: Align,
    /// 主轴分布。
    pub justify: Justify,
    /// Flex 增长因子。
    pub flex_grow: f32,
    /// Flex 收缩因子。
    pub flex_shrink: f32,
    /// Absolute / Anchor 用的局部偏移。
    pub offset_x: f32,
    /// Absolute / Anchor 用的局部纵偏。
    pub offset_y: f32,
    /// Grid 列数（`Layout::Grid`）。
    pub columns: u32,
}

impl Default for LayoutSpec {
    fn default() -> Self {
        Self {
            kind: Layout::Flex,
            direction: FlexDirection::Column,
            width: Size::Auto,
            height: Size::Auto,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            padding: Insets::default(),
            margin: Insets::default(),
            gap: 0.0,
            align: Align::Start,
            justify: Justify::Start,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            columns: 1,
        }
    }
}

impl LayoutSpec {
    /// 纵向 Flex 列布局。
    pub fn vertical() -> Self {
        Self { kind: Layout::Flex, direction: FlexDirection::Column, ..Self::default() }
    }

    /// 横向 Flex 行布局。
    pub fn horizontal() -> Self {
        Self { kind: Layout::Flex, direction: FlexDirection::Row, ..Self::default() }
    }

    /// Overlay 铺满式布局。
    pub fn overlay() -> Self {
        Self { kind: Layout::Overlay, ..Self::default() }
    }

    /// 指定列数的网格布局。
    pub fn grid(columns: u32) -> Self {
        Self { kind: Layout::Grid, columns: columns.max(1), ..Self::default() }
    }

    /// 设置子项间隙。
    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// 设置内边距。
    pub fn with_padding(mut self, padding: Insets) -> Self {
        self.padding = padding;
        self
    }

    /// 设置期望宽度。
    pub fn with_width(mut self, width: Size) -> Self {
        self.width = width;
        self
    }

    /// 设置期望高度。
    pub fn with_height(mut self, height: Size) -> Self {
        self.height = height;
        self
    }
}

/// 四边内/外边距。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Insets {
    /// 左边距。
    pub left: f32,
    /// 上边距。
    pub top: f32,
    /// 右边距。
    pub right: f32,
    /// 下边距。
    pub bottom: f32,
}

impl Insets {
    /// 四边同值。
    pub const fn all(v: f32) -> Self {
        Self { left: v, top: v, right: v, bottom: v }
    }

    /// 水平 / 垂直分别同值。
    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self { left: horizontal, top: vertical, right: horizontal, bottom: vertical }
    }

    /// 左右之和。
    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    /// 上下之和。
    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}

/// 布局引擎写出的节点几何（供命中测试与绘制）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ComputedLayout {
    /// 边框盒（含 padding，相对屏幕）。
    pub rect: Rect,
    /// 内容区（扣除 padding）。
    pub content_rect: Rect,
    /// 可选裁剪矩形（ScrollView 等）。
    pub clip_rect: Option<Rect>,
    /// 文本基线（相对 `rect` 顶边）。
    pub baseline: f32,
    /// measure 阶段缓存的期望尺寸（含 margin）。
    pub desired: super::Size2,
}
