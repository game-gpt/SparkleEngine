//! 无障碍语义树。

use crate::id::WidgetId;
use crate::node::WidgetKind;
use crate::tree::WidgetTree;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    None,
    Button,
    Label,
    TextField,
    Checkbox,
    Radio,
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
    pub value: Option<String>,
    pub checked: Option<bool>,
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

    /// 从 Widget 树重建语义节点（深度优先）。
    pub fn rebuild_from(&mut self, tree: &WidgetTree) {
        self.clear();
        rebuild_rec(self, tree, tree.root());
    }
}

fn rebuild_rec(out: &mut AccessibilityTree, tree: &WidgetTree, id: WidgetId) {
    let Some(node) = tree.node(id) else {
        return;
    };
    if !node.state.visible {
        return;
    }
    if let Some(role) = role_for(node.kind) {
        let label = node.content.text.clone().unwrap_or_default();
        let value = match node.kind {
            WidgetKind::Slider | WidgetKind::ProgressBar => Some(format!("{:.3}", node.content.value)),
            WidgetKind::TextField | WidgetKind::TextArea => node.content.text.clone(),
            _ => None,
        };
        let checked = matches!(
            node.kind,
            WidgetKind::Checkbox | WidgetKind::Toggle | WidgetKind::Radio
        )
        .then_some(node.content.checked);
        out.push(AccessibilityNode {
            id,
            role,
            label,
            value,
            checked,
            disabled: node.state.disabled,
            focused: node.state.focused,
            selected: node.state.selected,
        });
    }
    for child in &node.children {
        rebuild_rec(out, tree, *child);
    }
}

fn role_for(kind: WidgetKind) -> Option<Role> {
    Some(match kind {
        WidgetKind::Root | WidgetKind::Container | WidgetKind::Spacer | WidgetKind::Separator => {
            return None;
        }
        WidgetKind::Button => Role::Button,
        WidgetKind::Label => Role::Label,
        WidgetKind::TextField | WidgetKind::TextArea => Role::TextField,
        WidgetKind::Checkbox | WidgetKind::Toggle => Role::Checkbox,
        WidgetKind::Radio => Role::Radio,
        WidgetKind::Slider | WidgetKind::ProgressBar => Role::Slider,
        WidgetKind::ListView => Role::List,
        WidgetKind::ScrollView => Role::ScrollView,
        WidgetKind::Image => Role::Image,
        WidgetKind::Modal | WidgetKind::Popup => Role::Dialog,
        WidgetKind::Panel
        | WidgetKind::GridView
        | WidgetKind::TreeView
        | WidgetKind::TabView
        | WidgetKind::SplitView
        | WidgetKind::Tooltip
        | WidgetKind::Toast
        | WidgetKind::Custom => Role::Custom,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::{button_widget, checkbox_widget, column};

    #[test]
    fn rebuild_includes_interactive_widgets() {
        let mut tree = WidgetTree::new();
        let root = tree.root();
        column()
            .child(button_widget().text("Go"))
            .child(checkbox_widget().text("On").checked(true))
            .mount(&mut tree, root)
            .unwrap();
        let mut a11y = AccessibilityTree::default();
        a11y.rebuild_from(&tree);
        assert!(a11y.nodes().iter().any(|n| n.role == Role::Button && n.label == "Go"));
        assert!(a11y
            .nodes()
            .iter()
            .any(|n| n.role == Role::Checkbox && n.checked == Some(true)));
    }
}
