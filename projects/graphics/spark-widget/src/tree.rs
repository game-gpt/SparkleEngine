//! Retained Widget 树。

use std::collections::HashMap;

use crate::{
    id::WidgetId,
    node::{WidgetKind, WidgetNode},
};

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
        Self { root, nodes, next_id: 1 }
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
        let children = self.nodes.get(&id).map(|n| n.children.clone()).unwrap_or_default();
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
        let children = self.nodes.get(&id).map(|n| n.children.clone()).unwrap_or_default();
        for child in children {
            self.unmount(child);
        }
    }

    /// 所有节点 ID（无序）。
    pub fn ids(&self) -> Vec<WidgetId> {
        self.nodes.keys().copied().collect()
    }

    /// 直接子节点中匹配 `key` 的第一个。
    pub fn child_by_key(&self, parent: WidgetId, key: &str) -> Option<WidgetId> {
        let children = self.node(parent)?.children.clone();
        children.into_iter().find(|&id| {
            self.node(id)
                .and_then(|n| n.key.as_deref())
                .is_some_and(|k| k == key)
        })
    }

    /// 子树 DFS（含 `root` 自身）按 `key` 查找。
    pub fn find_by_key(&self, root: WidgetId, key: &str) -> Option<WidgetId> {
        if self
            .node(root)
            .and_then(|n| n.key.as_deref())
            .is_some_and(|k| k == key)
        {
            return Some(root);
        }
        let children = self.node(root)?.children.clone();
        for child in children {
            if let Some(id) = self.find_by_key(child, key) {
                return Some(id);
            }
        }
        None
    }

    /// `root` 下直接子节点里，`key` 以 `prefix` 开头的 ID（按挂载序）。
    pub fn children_with_key_prefix(&self, root: WidgetId, prefix: &str) -> Vec<WidgetId> {
        let Some(node) = self.node(root) else {
            return Vec::new();
        };
        node.children
            .iter()
            .copied()
            .filter(|&id| {
                self.node(id)
                    .and_then(|n| n.key.as_deref())
                    .is_some_and(|k| k.starts_with(prefix))
            })
            .collect()
    }
}
