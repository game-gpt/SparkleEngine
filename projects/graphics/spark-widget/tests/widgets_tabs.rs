//! 自 `src/widgets/tabs.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_types::Vec2;
use spark_widget::{
    layout::{UiMetrics, run_layout},
    text::EstimateMeasurer,
    widgets::label_widget,
};

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

#[test]
fn sync_tabs_reuses_tab_button_ids() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let id = tab_view().mount(&mut tree, root).unwrap();
    sync_tabs(&mut tree, id, 0, &[("One", label_widget().text("p0")), ("Two", label_widget().text("p1"))]).unwrap();
    let bar = tree.child_by_key(id, "tab-bar").unwrap();
    let tab0 = tree.child_by_key(bar, "tab-0").unwrap();
    let tab1 = tree.child_by_key(bar, "tab-1").unwrap();

    sync_tabs(&mut tree, id, 1, &[("One", label_widget().text("p0")), ("Two", label_widget().text("p1"))]).unwrap();
    assert_eq!(tree.child_by_key(bar, "tab-0"), Some(tab0));
    assert_eq!(tree.child_by_key(bar, "tab-1"), Some(tab1));
    assert!(tree.node(tab1).unwrap().state.selected);
}
