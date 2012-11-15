//! 布局规格与计算结果。

use spark_core::Rect;

use super::{Align, Size};

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
    pub flex_grow: f32,
    pub flex_shrink: f32,
}

impl Default for LayoutSpec {
    fn default() -> Self {
        Self {
            kind: Layout::Flex,
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
            flex_grow: 0.0,
            flex_shrink: 1.0,
        }
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
        Self {
            left: v,
            top: v,
            right: v,
            bottom: v,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ComputedLayout {
    pub rect: Rect,
    pub content_rect: Rect,
    pub clip_rect: Option<Rect>,
    pub baseline: f32,
}
