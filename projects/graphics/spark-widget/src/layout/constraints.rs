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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constraints {
    pub min: Size2,
    pub max: Size2,
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            min: Size2::default(),
            max: Size2 {
                width: f32::INFINITY,
                height: f32::INFINITY,
            },
        }
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
