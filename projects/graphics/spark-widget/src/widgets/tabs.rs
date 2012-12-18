//! TabView：页签栏 + 页面切换。

use crate::{
    id::WidgetId,
    layout::{LayoutSpec, Size},
    node::WidgetKind,
    tree::WidgetTree,
    widgets::{WidgetBuilder, button_widget, column, row},
};

/// TabView 根：纵向 = 页签栏 + 内容区。
pub fn tab_view() -> WidgetBuilder {
    WidgetBuilder::new(WidgetKind::TabView).layout(LayoutSpec { width: Size::Fill, height: Size::Fill, ..LayoutSpec::vertical() })
}

/// 根据 `selected` 重建 TabView 子树。
///
/// 结构：
/// ```text
/// TabView
///   tab_bar (row)
///     Button × N  (content.value = index)
///   pages (column fill)
///     page widgets（仅选中页 visible）
/// ```
pub fn sync_tabs(tree: &mut WidgetTree, tab_view_id: WidgetId, selected: usize, tabs: &[(&str, WidgetBuilder)]) -> Option<WidgetId> {
    let kind = tree.node(tab_view_id).map(|n| n.kind)?;
    if kind != WidgetKind::TabView {
        return None;
    }
    let selected = selected.min(tabs.len().saturating_sub(1));
    if let Some(node) = tree.node_mut(tab_view_id) {
        node.content.value = selected as f32;
    }

    tree.clear_children(tab_view_id);

    let mut bar = row().key("tab-bar").layout(LayoutSpec { width: Size::Fill, height: Size::Px(36.0), gap: 4.0, ..LayoutSpec::horizontal() });
    for (index, (title, _)) in tabs.iter().enumerate() {
        let selected_tab = index == selected;
        bar = bar.child(
            button_widget()
                .key(format!("tab-{index}"))
                .text(*title)
                .value(index as f32)
                .layout(LayoutSpec { height: Size::Px(32.0), flex_grow: 1.0, ..LayoutSpec::horizontal() })
                .style(crate::style::Style { opacity: Some(if selected_tab { 1.0 } else { 0.75 }), ..crate::style::Style::default() }),
        );
    }
    bar.mount(tree, tab_view_id)?;

    let mut pages = column().key("tab-pages").layout(LayoutSpec { width: Size::Fill, height: Size::Fill, ..LayoutSpec::vertical() });
    for (index, (_, page)) in tabs.iter().enumerate() {
        pages = pages.child(page.clone().key(format!("page-{index}")));
    }
    let pages_id = pages.mount(tree, tab_view_id)?;

    let page_ids = tree.node(pages_id).map(|n| n.children.clone()).unwrap_or_default();
    for (index, page_id) in page_ids.into_iter().enumerate() {
        if let Some(node) = tree.node_mut(page_id) {
            node.state.visible = index == selected;
        }
    }
    Some(tab_view_id)
}

/// 若点击落在 TabView 页签按钮上，切换选中页并返回新索引。
pub fn handle_tab_click(tree: &mut WidgetTree, clicked: WidgetId) -> Option<usize> {
    let (tab_view, index) = {
        let node = tree.node(clicked)?;
        if node.kind != WidgetKind::Button {
            return None;
        }
        let index = node.content.value as usize;
        let mut parent = node.parent?;
        loop {
            let p = tree.node(parent)?;
            if p.kind == WidgetKind::TabView {
                break (parent, index);
            }
            parent = p.parent?;
        }
    };
    let page_count =
        tree.node(tab_view).and_then(|n| n.children.get(1).copied()).and_then(|pages| tree.node(pages).map(|p| p.children.len())).unwrap_or(0);
    if page_count == 0 || index >= page_count {
        return None;
    }
    if let Some(node) = tree.node_mut(tab_view) {
        node.content.value = index as f32;
    }
    let pages = tree.node(tab_view)?.children.get(1).copied()?;
    let page_ids = tree.node(pages)?.children.clone();
    for (i, page_id) in page_ids.into_iter().enumerate() {
        if let Some(node) = tree.node_mut(page_id) {
            node.state.visible = i == index;
        }
    }
    // 刷新页签按钮透明度。
    let bar = tree.node(tab_view)?.children.first().copied()?;
    let tab_ids = tree.node(bar)?.children.clone();
    for (i, tab_id) in tab_ids.into_iter().enumerate() {
        if let Some(node) = tree.node_mut(tab_id) {
            node.style.opacity = Some(if i == index { 1.0 } else { 0.75 });
            node.state.selected = i == index;
        }
    }
    Some(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layout::{UiMetrics, run_layout},
        text::EstimateMeasurer,
        widgets::label_widget,
    };
    use spark_core::Vec2;

    #[test]
    fn sync_tabs_shows_only_selected_page() {
        let mut tree = WidgetTree::new();
        let root = tree.root();
        let id = tab_view().mount(&mut tree, root).unwrap();
        sync_tabs(
            &mut tree,
            id,
            1,
            &[("One", label_widget().text("p0")), ("Two", label_widget().text("p1")), ("Three", label_widget().text("p2"))],
        )
        .unwrap();
        run_layout(&mut tree, Vec2::new(300.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
        let pages = tree.node(id).unwrap().children[1];
        let kids = &tree.node(pages).unwrap().children;
        assert!(!tree.node(kids[0]).unwrap().state.visible);
        assert!(tree.node(kids[1]).unwrap().state.visible);
        assert!(!tree.node(kids[2]).unwrap().state.visible);
    }
}
