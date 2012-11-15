//! 无障碍语义树（占位）。

use crate::id::WidgetId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    None,
    Button,
    Label,
    TextField,
    Checkbox,
    Slider,
    List,
    ListItem,
    Dialog,
    Menu,
    MenuItem,
    ScrollView,
    Image,
    Custom,
}

#[derive(Debug, Clone)]
pub struct AccessibilityNode {
    pub id: WidgetId,
    pub role: Role,
    pub label: String,
    pub disabled: bool,
    pub focused: bool,
    pub selected: bool,
}

#[derive(Debug, Default)]
pub struct AccessibilityTree {
    nodes: Vec<AccessibilityNode>,
}

impl AccessibilityTree {
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    pub fn push(&mut self, node: AccessibilityNode) {
        self.nodes.push(node);
    }

    pub fn nodes(&self) -> &[AccessibilityNode] {
        &self.nodes
    }
}
