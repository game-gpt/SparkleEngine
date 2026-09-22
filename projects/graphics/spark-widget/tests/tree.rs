//! 自 `src/tree.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_widget::{
    node::WidgetKind,
    widgets::{button_widget, column},
};

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
    let id = column().key("menu").child(button_widget().key("start")).mount(&mut tree, root).unwrap();
    assert_eq!(tree.node(id).unwrap().children.len(), 1);
}

#[test]
fn find_by_key_and_child_by_key() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let menu = column()
        .key("menu")
        .child(button_widget().key("start"))
        .child(button_widget().key("quit"))
        .mount(&mut tree, root)
        .unwrap();
    assert_eq!(tree.find_by_key(root, "start"), tree.child_by_key(menu, "start"));
    assert_eq!(
        tree.children_with_key_prefix(menu, "q").len(),
        1
    );
}
