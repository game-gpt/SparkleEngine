//! 声明式控件构建。

use crate::id::WidgetId;
use crate::layout::{FlexDirection, LayoutSpec};
use crate::node::{WidgetContent, WidgetKind};
use crate::style::Style;
use crate::tree::WidgetTree;

/// 轻量 builder：往树里挂节点。
#[derive(Debug, Clone)]
pub struct WidgetBuilder {
    kind: WidgetKind,
    key: Option<String>,
    style: Style,
    layout: LayoutSpec,
    content: WidgetContent,
    focusable: Option<bool>,
    children: Vec<WidgetBuilder>,
}

impl WidgetBuilder {
    pub fn new(kind: WidgetKind) -> Self {
        Self {
            kind,
            key: None,
            style: Style::default(),
            layout: LayoutSpec::default(),
            content: WidgetContent::default(),
            focusable: None,
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

    pub fn layout(mut self, layout: LayoutSpec) -> Self {
        self.layout = layout;
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.content.text = Some(text.into());
        self
    }

    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = Some(focusable);
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
            node.layout = self.layout;
            node.content = self.content;
            if let Some(focusable) = self.focusable {
                node.focusable = focusable;
            }
        }
        for child in self.children {
            child.mount(tree, id);
        }
        Some(id)
    }
}

pub fn column() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Container).layout(LayoutSpec::vertical())
}

pub fn row() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Container).layout(LayoutSpec {
        direction: FlexDirection::Row,
        ..LayoutSpec::horizontal()
    })
}

pub fn label_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Label)
}

pub fn button_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Button)
}

pub fn overlay_root() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Container).layout(LayoutSpec::overlay())
}
