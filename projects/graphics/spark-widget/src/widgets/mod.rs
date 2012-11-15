//! 声明式控件构建（占位）。

use crate::id::WidgetId;
use crate::node::WidgetKind;
use crate::style::Style;
use crate::tree::WidgetTree;

/// 轻量 builder：往树里挂节点。逻辑填充前只建立结构。
#[derive(Debug, Clone)]
pub struct WidgetBuilder {
    kind: WidgetKind,
    key: Option<String>,
    style: Style,
    children: Vec<WidgetBuilder>,
}

impl WidgetBuilder {
    pub fn new(kind: WidgetKind) -> Self {
        Self {
            kind,
            key: None,
            style: Style::default(),
            children: Vec::new(),
        }
    }

    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn child(mut self, child: WidgetBuilder) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = WidgetBuilder>) -> Self {
        self.children.extend(children);
        self
    }

    /// 挂到 parent 下，返回本节点 ID。
    pub fn mount(self, tree: &mut WidgetTree, parent: WidgetId) -> Option<WidgetId> {
        let id = tree.mount(parent, self.kind)?;
        if let Some(node) = tree.node_mut(id) {
            node.key = self.key;
            node.style = self.style;
        }
        for child in self.children {
            child.mount(tree, id);
        }
        Some(id)
    }
}

pub fn column() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Container)
}

pub fn row() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Container)
}

pub fn label_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Label)
}

pub fn button_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Button)
}

pub fn overlay_root() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Container)
}
