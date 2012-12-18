//! 布局规格与计算结果。

use spark_core::Rect;

use super::{Align, FlexDirection, Justify, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Stack,
    Flex,
    Grid,
    Overlay,
    Anchor,
    Absolute,
}

impl Default for Layout {
    fn default() -> Self {
        Self::Flex
    }
}

#[derive(Debug, Clone)]
pub struct LayoutSpec {
    pub kind: Layout,
    pub direction: FlexDirection,
    pub width: Size,
    pub height: Size,
    pub min_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
    pub padding: Insets,
    pub margin: Insets,
    pub gap: f32,
    pub align: Align,
    pub justify: Justify,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    /// Absolute / Anchor 用的局部偏移。
    pub offset_x: f32,
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
    pub fn vertical() -> Self {
        Self { kind: Layout::Flex, direction: FlexDirection::Column, ..Self::default() }
    }

    pub fn horizontal() -> Self {
        Self { kind: Layout::Flex, direction: FlexDirection::Row, ..Self::default() }
    }

    pub fn overlay() -> Self {
        Self { kind: Layout::Overlay, ..Self::default() }
    }

    pub fn grid(columns: u32) -> Self {
        Self { kind: Layout::Grid, columns: columns.max(1), ..Self::default() }
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn with_padding(mut self, padding: Insets) -> Self {
        self.padding = padding;
        self
    }

    pub fn with_width(mut self, width: Size) -> Self {
        self.width = width;
        self
    }

    pub fn with_height(mut self, height: Size) -> Self {
        self.height = height;
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Insets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Insets {
    pub const fn all(v: f32) -> Self {
        Self { left: v, top: v, right: v, bottom: v }
    }

    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self { left: horizontal, top: vertical, right: horizontal, bottom: vertical }
    }

    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ComputedLayout {
    pub rect: Rect,
    pub content_rect: Rect,
    pub clip_rect: Option<Rect>,
    pub baseline: f32,
    /// measure 阶段缓存的期望尺寸（含 margin）。
    pub desired: super::Size2,
}
