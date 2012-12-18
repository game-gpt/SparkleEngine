//! 尺寸与约束。

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    Auto,
    Px(f32),
    Percent(f32),
    Fill,
    MinContent,
    MaxContent,
}

impl Default for Size {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Size2 {
    pub width: f32,
    pub height: f32,
}

impl Size2 {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn clamp(self, constraints: Constraints) -> Self {
        Self {
            width: self.width.clamp(constraints.min.width, constraints.max.width),
            height: self.height.clamp(constraints.min.height, constraints.max.height),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constraints {
    pub min: Size2,
    pub max: Size2,
}

impl Default for Constraints {
    fn default() -> Self {
        Self { min: Size2::default(), max: Size2 { width: f32::INFINITY, height: f32::INFINITY } }
    }
}

impl Constraints {
    pub fn tight(size: Size2) -> Self {
        Self { min: size, max: size }
    }

    pub fn loose(max: Size2) -> Self {
        Self { min: Size2::default(), max }
    }

    pub fn with_max_width(mut self, width: f32) -> Self {
        self.max.width = self.max.width.min(width).max(self.min.width);
        self
    }

    pub fn with_max_height(mut self, height: f32) -> Self {
        self.max.height = self.max.height.min(height).max(self.min.height);
        self
    }

    pub fn deflate(self, insets: super::Insets) -> Self {
        let width = (self.max.width - insets.horizontal()).max(0.0);
        let height = (self.max.height - insets.vertical()).max(0.0);
        let min_w = self.min.width.min(width);
        let min_h = self.min.height.min(height);
        Self { min: Size2::new(min_w, min_h), max: Size2::new(width, height) }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FlexDirection {
    #[default]
    Column,
    Row,
}
