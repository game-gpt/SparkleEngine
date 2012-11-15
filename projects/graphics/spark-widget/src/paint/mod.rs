//! Paint traversal：retained 树 → DrawList。

use spark_renderer::DrawList;

use crate::node::WidgetKind;
use crate::style::{ComputedStyle, Theme};
use crate::tree::WidgetTree;

/// 遍历树并写入绘制命令（占位：仅画有背景的节点矩形）。
pub fn paint_tree(tree: &WidgetTree, theme: &Theme, draw: &mut DrawList) {
    paint_node(tree, theme, draw, tree.root());
}

fn paint_node(tree: &WidgetTree, theme: &Theme, draw: &mut DrawList, id: crate::id::WidgetId) {
    let Some(node) = tree.node(id) else {
        return;
    };
    if !node.state.visible {
        return;
    }
    let style = ComputedStyle::resolve(theme, &node.style);
    if node.kind != WidgetKind::Root {
        let mut color = style.background;
        color.a *= style.opacity;
        draw.fill_rect(node.computed.rect, color);
    }
    for child in &node.children {
        paint_node(tree, theme, draw, *child);
    }
}

/// 绘制上下文占位，供自定义 Widget 使用。
pub struct PaintContext<'a> {
    pub draw: &'a mut DrawList,
    pub theme: &'a Theme,
}
