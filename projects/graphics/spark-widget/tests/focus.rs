//! 自 `src/focus/mod.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_widget::{
    layout::{LayoutSpec, Size},
    widgets::{button_widget, column, modal_widget},
};

#[test]
fn trap_cycles_only_inside_modal() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let outside = button_widget().text("out").mount(&mut tree, root).unwrap();
    let modal = modal_widget()
        .child(
            column()
                .layout(LayoutSpec { width: Size::Px(200.0), height: Size::Px(120.0), ..LayoutSpec::default() })
                .child(button_widget().text("a"))
                .child(button_widget().text("b")),
        )
        .mount(&mut tree, root)
        .unwrap();

    let list = collect_focusable_in(&tree, modal);
    assert_eq!(list.len(), 2);
    assert!(!list.contains(&outside));

    let mut focus = FocusManager { focused: Some(outside) };
    ensure_focus_in_trap(&tree, &mut focus, modal);
    assert_eq!(focus.focused, Some(list[0]));

    focus_next_in(&tree, &mut focus, Some(modal));
    assert_eq!(focus.focused, Some(list[1]));
    focus_next_in(&tree, &mut focus, Some(modal));
    assert_eq!(focus.focused, Some(list[0]));
}

#[test]
fn tab_index_orders_positive_before_document_zero() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let a = button_widget().text("a").tab_index(0).mount(&mut tree, root).unwrap();
    let b = button_widget().text("b").tab_index(2).mount(&mut tree, root).unwrap();
    let c = button_widget().text("c").tab_index(1).mount(&mut tree, root).unwrap();
    let skip = button_widget().text("skip").tab_index(-1).mount(&mut tree, root).unwrap();

    let list = collect_focusable(&tree);
    assert_eq!(list, vec![c, b, a]);
    assert!(!list.contains(&skip));
    assert!(tree.node(skip).unwrap().focusable);
}

#[test]
fn focus_policy_applies_neighbors_and_tab_index() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let first = button_widget().text("first").mount(&mut tree, root).unwrap();
    let second = button_widget()
        .text("second")
        .focus_policy(FocusPolicy { focusable: true, tab_index: 5, neighbors: Neighbors { up: Some(first), ..Neighbors::default() } })
        .mount(&mut tree, root)
        .unwrap();

    let node = tree.node(second).unwrap();
    assert_eq!(node.tab_index, 5);
    assert_eq!(node.neighbors.up, Some(first));
    assert_eq!(collect_focusable(&tree), vec![second, first]);
}
