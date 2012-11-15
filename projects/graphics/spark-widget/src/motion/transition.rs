//! 样式属性过渡占位。

use super::Easing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleProperty {
    Opacity,
    Scale,
    Translation,
    Color,
}

#[derive(Debug, Clone, Copy)]
pub struct Transition {
    pub property: StyleProperty,
    pub duration: f32,
    pub easing: Easing,
}
