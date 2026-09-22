//! 无障碍语义树。
//!
//! 从 retained [`WidgetTree`](crate::tree::WidgetTree) 抽取可读角色与状态，
//! 供屏幕阅读器 / 调试观察；不参与布局或绘制。

use crate::{id::WidgetId, node::WidgetKind, tree::WidgetTree};

/// 控件在无障碍语义中的角色（由 [`WidgetKind`] 映射，非一一对应）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// 无语义角色（一般不进入语义树）。
    None,
    /// 可激活按钮。
    Button,
    /// 静态文本标签。
    Label,
    /// 可编辑文本框。
    TextField,
    /// 复选框 / 开关类。
    Checkbox,
    /// 单选按钮。
    Radio,
    /// 滑条或进度类可调数值。
    Slider,
    /// 列表容器。
    List,
    /// 列表项（预留）。
    ListItem,
    /// 对话框 / 模态层。
    Dialog,
    /// 菜单容器（预留）。
    Menu,
    /// 菜单项（预留）。
    MenuItem,
    /// 可滚动视口。
    ScrollView,
    /// 图片。
    Image,
    /// 未细分到标准角色的控件。
    Custom,
}

/// 语义树中的一个节点：对应可见且有角色的 Widget。
#[derive(Debug, Clone)]
pub struct AccessibilityNode {
    /// 源控件稳定 ID。
    pub id: WidgetId,
    /// 语义角色。
    pub role: Role,
    /// 可读标签（通常取自 `WidgetContent::text`）。
    pub label: String,
    /// 当前值的文本表示（滑条数值、输入框文本等）。
    pub value: Option<String>,
    /// 勾选态；非勾选类控件为 `None`。
    pub checked: Option<bool>,
    /// 是否禁用。
    pub disabled: bool,
    /// 是否持有焦点。
    pub focused: bool,
    /// 是否选中（列表行 / Tab 等）。
    pub selected: bool,
}

/// 扁平化的无障碍语义树（深度优先顺序）。
#[derive(Debug, Default)]
pub struct AccessibilityTree {
    nodes: Vec<AccessibilityNode>,
}

impl AccessibilityTree {
    /// 清空全部语义节点。
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    /// 追加一个语义节点。
    pub fn push(&mut self, node: AccessibilityNode) {
        self.nodes.push(node);
    }

    /// 只读访问当前语义节点切片。
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
    let Some(node) = tree.node(id)
    else {
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
        let checked = matches!(node.kind, WidgetKind::Checkbox | WidgetKind::Toggle | WidgetKind::Radio).then_some(node.content.checked);
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
