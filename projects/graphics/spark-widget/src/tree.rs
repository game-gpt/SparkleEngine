//! Retained Widget 树。

use std::collections::HashMap;

use crate::id::WidgetId;
use crate::node::{WidgetKind, WidgetNode};

#[derive(Debug)]
pub struct WidgetTree {
    root: WidgetId,
    nodes: HashMap<WidgetId, WidgetNode>,
    next_id: u64,
}

impl Default for WidgetTree {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetTree {
    pub fn new() -> Self {
        let root = WidgetId::ROOT;
        let mut nodes = HashMap::new();
        nodes.insert(root, WidgetNode::new(root, WidgetKind::Root));
        Self {
            root,
            nodes,
            next_id: 1,
        }
    }

    pub fn root(&self) -> WidgetId {
        self.root
    }

    pub fn node(&self, id: WidgetId) -> Option<&WidgetNode> {
        self.nodes.get(&id)
    }

    pub fn node_mut(&mut self, id: WidgetId) -> Option<&mut WidgetNode> {
        self.nodes.get_mut(&id)
    }

    pub fn alloc_id(&mut self) -> WidgetId {
        let id = WidgetId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    /// 挂载子节点。返回新节点 ID。
    pub fn mount(&mut self, parent: WidgetId, kind: WidgetKind) -> Option<WidgetId> {
        if !self.nodes.contains_key(&parent) {
            return None;
        }
        let id = self.alloc_id();
        let mut node = WidgetNode::new(id, kind);
        node.parent = Some(parent);
        self.nodes.insert(id, node);
        if let Some(p) = self.nodes.get_mut(&parent) {
            p.children.push(id);
        }
        Some(id)
    }

    /// 卸载子树（含自身）。
    pub fn unmount(&mut self, id: WidgetId) {
        if id == self.root {
            return;
        }
        let children = self
            .nodes
            .get(&id)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        for child in children {
            self.unmount(child);
        }
        if let Some(node) = self.nodes.remove(&id) {
            if let Some(parent) = node.parent {
                if let Some(p) = self.nodes.get_mut(&parent) {
                    p.children.retain(|c| *c != id);
                }
            }
        }
    }

    pub fn clear_children(&mut self, id: WidgetId) {
        let children = self
            .nodes
            .get(&id)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        for child in children {
            self.unmount(child);
        }
    }

    /// 所有节点 ID（无序）。
    pub fn ids(&self) -> Vec<WidgetId> {
        self.nodes.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::WidgetKind;
    use crate::widgets::{button_widget, column};

    #[test]
    fn mount_and_unmount_subtree() {
        let mut tree = WidgetTree::new();
        let root = tree.root();
        let panel = tree.mount(root, WidgetKind::Container).unwrap();
        let btn = tree.mount(panel, WidgetKind::Button).unwrap();
        assert!(tree.node(btn).is_some());
        tree.unmount(panel);
        assert!(tree.node(panel).is_none());
        assert!(tree.node(btn).is_none());
        assert!(tree.node(root).unwrap().children.is_empty());
    }

    #[test]
    fn builder_mounts_children() {
        let mut tree = WidgetTree::new();
        let root = tree.root();
        let id = column()
            .key("menu")
            .child(button_widget().key("start"))
            .mount(&mut tree, root)
            .unwrap();
        assert_eq!(tree.node(id).unwrap().children.len(), 1);
    }
}
