//! 约束式布局：measure / arrange。

mod constraints;
mod engine;
mod spec;

pub use constraints::{Align, Constraints, FlexDirection, Justify, Size, Size2};
pub use engine::run_layout;
pub use spec::{ComputedLayout, Insets, Layout, LayoutSpec};
