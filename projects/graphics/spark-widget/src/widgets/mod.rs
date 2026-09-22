//! 声明式控件构建。

mod list;
mod tabs;

pub use list::{content_height, sync_visible_rows, visible_row_range};
pub use tabs::{handle_tab_click, sync_tabs, tab_view};

use crate::{
    id::WidgetId,
    layout::{FlexDirection, LayoutSpec, Size},
    node::{WidgetContent, WidgetKind},
    style::Style,
    tree::WidgetTree,
};

/// 轻量 builder：往树里挂节点。
#[derive(Debug, Clone)]
pub struct WidgetBuilder {
    kind: WidgetKind,
    key: Option<String>,
    style: Style,
    layout: LayoutSpec,
    content: WidgetContent,
    focusable: Option<bool>,
    tab_index: Option<i32>,
    neighbors: Option<crate::focus::Neighbors>,
    layer: Option<crate::runtime::UiLayer>,
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
            tab_index: None,
            neighbors: None,
            layer: None,
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

    pub fn on_click(mut self, command: crate::command::UiCommand) -> Self {
        self.content.click_command = Some(command);
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.content.checked = checked;
        self
    }

    pub fn value(mut self, value: f32) -> Self {
        self.content.value = value;
        self
    }

    pub fn value_range(mut self, min: f32, max: f32) -> Self {
        self.content.value_min = min;
        self.content.value_max = max;
        self
    }

    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = Some(focusable);
        self
    }

    pub fn tab_index(mut self, tab_index: i32) -> Self {
        self.tab_index = Some(tab_index);
        self
    }

    pub fn neighbors(mut self, neighbors: crate::focus::Neighbors) -> Self {
        self.neighbors = Some(neighbors);
        self
    }

    /// 一次设置 `focusable` / `tab_index` / `neighbors`。
    pub fn focus_policy(mut self, policy: crate::focus::FocusPolicy) -> Self {
        self.focusable = Some(policy.focusable);
        self.tab_index = Some(policy.tab_index);
        self.neighbors = Some(policy.neighbors);
        self
    }

    pub fn drag_source(mut self, enabled: bool) -> Self {
        self.content.drag_source = enabled;
        self
    }

    pub fn drop_target(mut self, enabled: bool) -> Self {
        self.content.drop_target = enabled;
        self
    }

    pub fn image(mut self, image: crate::asset::UiImage) -> Self {
        self.content.image = Some(image);
        self
    }

    pub fn stop_click_propagation(mut self, stop: bool) -> Self {
        self.content.stop_click_propagation = stop;
        self
    }

    pub fn prevent_click_default(mut self, prevent: bool) -> Self {
        self.content.prevent_click_default = prevent;
        self
    }

    pub fn layer(mut self, layer: crate::runtime::UiLayer) -> Self {
        self.layer = Some(layer);
        self
    }

    /// 控件种类。
    pub fn kind(&self) -> WidgetKind {
        self.kind
    }

    /// 稳定键（若有）。
    pub fn key_str(&self) -> Option<&str> {
        self.key.as_deref()
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
        let inherited = tree.node(parent).map(|n| n.layer);
        let id = tree.mount(parent, self.kind)?;
        self.apply_to(tree, id, inherited);
        for child in self.children {
            child.mount(tree, id);
        }
        Some(id)
    }

    /// 按 `key` + `kind` 复用 parent 下已有子节点；无匹配则 `mount`。
    ///
    /// 复用时保留 `WidgetId`、滚动偏移与文本光标（文案未变时），并递归 reconcile 子树。
    /// 无 key 的节点按「同 kind 且尚未占用」的文档序匹配。
    pub fn reconcile(self, tree: &mut WidgetTree, parent: WidgetId) -> Option<WidgetId> {
        self.reconcile_excluding(tree, parent, &std::collections::HashSet::new())
    }

    fn reconcile_excluding(
        self,
        tree: &mut WidgetTree,
        parent: WidgetId,
        parent_claimed: &std::collections::HashSet<WidgetId>,
    ) -> Option<WidgetId> {
        if tree.node(parent).is_none() {
            return None;
        }
        let inherited = tree.node(parent).map(|n| n.layer);
        let existing =
            find_reconcile_match(tree, parent, self.kind, self.key.as_deref(), parent_claimed);
        let id = if let Some(id) = existing {
            self.apply_to(tree, id, inherited);
            id
        } else {
            let id = tree.mount(parent, self.kind)?;
            self.apply_to(tree, id, inherited);
            id
        };

        let old_children = tree.node(id).map(|n| n.children.clone()).unwrap_or_default();
        let mut used = std::collections::HashSet::new();
        let mut order = Vec::with_capacity(self.children.len());
        for child in self.children {
            if let Some(cid) = child.reconcile_excluding(tree, id, &used) {
                used.insert(cid);
                order.push(cid);
            }
        }
        for old in old_children {
            if !used.contains(&old) {
                tree.unmount(old);
            }
        }
        tree.set_child_order(id, &order);
        Some(id)
    }

    fn apply_to(&self, tree: &mut WidgetTree, id: WidgetId, inherited: Option<crate::runtime::UiLayer>) {
        let Some(node) = tree.node_mut(id)
        else {
            return;
        };
        let prev_text = node.content.text.clone();
        let prev_cursor = node.content.cursor;
        let prev_sel = node.content.sel_anchor;
        let prev_composition = node.content.composition.clone();
        let prev_scroll = node.scroll.clone();

        node.key = self.key.clone();
        node.style = self.style.clone();
        node.layout = self.layout.clone();
        node.content = self.content.clone();
        // 文案未变时保留编辑态，避免 reconcile 打断输入。
        if node.content.text == prev_text {
            node.content.cursor = prev_cursor;
            node.content.sel_anchor = prev_sel;
            node.content.composition = prev_composition;
        }
        node.state.checked = node.content.checked;
        if let Some(focusable) = self.focusable {
            node.focusable = focusable;
        }
        if let Some(tab_index) = self.tab_index {
            node.tab_index = tab_index;
        }
        if let Some(neighbors) = self.neighbors.clone() {
            node.neighbors = neighbors;
        }
        node.layer = self.layer.or(inherited).unwrap_or(crate::runtime::UiLayer::Gui);
        // ScrollView 等：保留运行时滚动，不被声明式默认冲掉。
        node.scroll = prev_scroll;
    }
}

fn find_reconcile_match(
    tree: &WidgetTree,
    parent: WidgetId,
    kind: WidgetKind,
    key: Option<&str>,
    claimed: &std::collections::HashSet<WidgetId>,
) -> Option<WidgetId> {
    let children = tree.node(parent)?.children.clone();
    if let Some(key) = key {
        return children.into_iter().find(|&id| {
            !claimed.contains(&id)
                && tree
                    .node(id)
                    .is_some_and(|n| n.kind == kind && n.key.as_deref() == Some(key))
        });
    }
    // 无 key：取第一个同 kind、也无 key、且尚未占用的子节点。
    children.into_iter().find(|&id| {
        !claimed.contains(&id)
            && tree
                .node(id)
                .is_some_and(|n| n.kind == kind && n.key.is_none())
    })
}

pub fn column() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Container).layout(LayoutSpec::vertical())
}

pub fn row() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Container).layout(LayoutSpec { direction: FlexDirection::Row, ..LayoutSpec::horizontal() })
}

pub fn panel() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Panel).layout(LayoutSpec::vertical())
}

pub fn label_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Label)
}

pub fn image_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Image).layout(LayoutSpec { width: Size::Px(32.0), height: Size::Px(32.0), ..LayoutSpec::default() })
}

pub fn button_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Button)
}

pub fn checkbox_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Checkbox).layout(LayoutSpec { height: Size::Px(24.0), ..LayoutSpec::horizontal() })
}

pub fn toggle_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Toggle).layout(LayoutSpec { height: Size::Px(24.0), ..LayoutSpec::horizontal() })
}

pub fn radio_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Radio).layout(LayoutSpec { height: Size::Px(24.0), ..LayoutSpec::horizontal() })
}

pub fn slider_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Slider).layout(LayoutSpec { width: Size::Fill, height: Size::Px(24.0), ..LayoutSpec::horizontal() })
}

pub fn progress_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::ProgressBar).layout(LayoutSpec { width: Size::Fill, height: Size::Px(12.0), ..LayoutSpec::horizontal() })
}

pub fn text_field_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::TextField).layout(LayoutSpec { width: Size::Fill, height: Size::Px(32.0), ..LayoutSpec::horizontal() })
}

pub fn separator_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Separator).layout(LayoutSpec { width: Size::Fill, height: Size::Px(1.0), ..LayoutSpec::default() })
}

pub fn spacer_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Spacer).layout(LayoutSpec { flex_grow: 1.0, ..LayoutSpec::default() })
}

pub fn grid(columns: u32) -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Container).layout(LayoutSpec::grid(columns).with_gap(8.0))
}

pub fn scroll_view() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::ScrollView).layout(LayoutSpec { width: Size::Fill, height: Size::Fill, ..LayoutSpec::vertical() })
}

/// 虚拟列表容器：底层是 `ScrollView`，行实例化由调用方按 [`visible_row_range`] 同步。
pub fn list_view() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::ListView).layout(LayoutSpec { width: Size::Fill, height: Size::Fill, ..LayoutSpec::vertical() })
}

pub fn modal_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Modal).layout(LayoutSpec {
        width: Size::Fill,
        height: Size::Fill,
        kind: crate::layout::Layout::Overlay,
        align: crate::layout::Align::Center,
        ..LayoutSpec::default()
    })
}

pub fn tooltip_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Tooltip).layout(LayoutSpec {
        width: Size::Auto,
        height: Size::Auto,
        padding: crate::layout::Insets::all(8.0),
        ..LayoutSpec::vertical()
    })
}

pub fn popup_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Popup).layout(LayoutSpec {
        width: Size::Auto,
        height: Size::Auto,
        padding: crate::layout::Insets::all(8.0),
        ..LayoutSpec::vertical()
    })
}

pub fn toast_widget() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Toast).layout(LayoutSpec {
        width: Size::Auto,
        height: Size::Auto,
        padding: crate::layout::Insets::symmetric(16.0, 10.0),
        ..LayoutSpec::vertical()
    })
}

pub fn overlay_root() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Container).layout(LayoutSpec::overlay())
}

/// HUD 场景根：默认 `UiLayer::Hud`，全屏 overlay 布局。
pub fn hud_root() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::Container).layer(crate::runtime::UiLayer::Hud).layout(LayoutSpec {
        width: Size::Fill,
        height: Size::Fill,
        kind: crate::layout::Layout::Overlay,
        ..LayoutSpec::default()
    })
}
