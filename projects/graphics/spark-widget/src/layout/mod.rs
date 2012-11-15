//! 约束式布局：measure / arrange。

mod constraints;
mod spec;

pub use constraints::{Align, Constraints, Size, Size2};
pub use spec::{ComputedLayout, Insets, Layout, LayoutSpec};

use spark_core::Vec2;

use crate::tree::WidgetTree;

/// 跑一遍布局（占位：把根填满屏幕，子节点尚未真正 measure/arrange）。
pub fn run_layout(tree: &mut WidgetTree, screen: Vec2, _dpi_scale: f32) {
    let root = tree.root();
    if let Some(node) = tree.node_mut(root) {
        node.computed.rect = spark_core::Rect::new(0.0, 0.0, screen.x, screen.y);
        node.computed.content_rect = node.computed.rect;
    }
    // TODO: 深度优先 measure → arrange。
}
