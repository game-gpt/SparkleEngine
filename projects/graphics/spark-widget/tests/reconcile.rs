//! `WidgetBuilder::reconcile` / `UiRuntime::reconcile_scene` 稳定性。

use spark_widget::{UiRuntime, WidgetTree, button_widget, column, label_widget};

#[test]
fn reconcile_reuses_ids_by_key() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let first = column()
        .key("menu")
        .child(button_widget().key("a").text("A"))
        .child(button_widget().key("b").text("B"))
        .mount(&mut tree, root)
        .unwrap();
    let a0 = tree.child_by_key(first, "a").unwrap();
    let b0 = tree.child_by_key(first, "b").unwrap();

    let second = column()
        .key("menu")
        .child(button_widget().key("b").text("B2"))
        .child(button_widget().key("a").text("A2"))
        .child(button_widget().key("c").text("C"))
        .reconcile(&mut tree, root)
        .unwrap();

    assert_eq!(second, first);
    assert_eq!(tree.child_by_key(second, "a"), Some(a0));
    assert_eq!(tree.child_by_key(second, "b"), Some(b0));
    assert!(tree.child_by_key(second, "c").is_some());
    assert_eq!(tree.node(a0).unwrap().content.text.as_deref(), Some("A2"));
    assert_eq!(tree.node(second).unwrap().children, vec![b0, a0, tree.child_by_key(second, "c").unwrap(),]);
}

#[test]
fn reconcile_drops_removed_keys() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let menu = column().key("menu").child(button_widget().key("a")).child(button_widget().key("gone")).mount(&mut tree, root).unwrap();
    let gone = tree.child_by_key(menu, "gone").unwrap();

    column().key("menu").child(button_widget().key("a")).reconcile(&mut tree, root).unwrap();

    assert!(tree.node(gone).is_none());
    assert_eq!(tree.node(menu).unwrap().children.len(), 1);
}

#[test]
fn reconcile_scene_preserves_hover_key() {
    let mut runtime = UiRuntime::new();
    let id = runtime
        .mount_scene(
            column().key("title-menu").child(button_widget().key("title-0").text("单人")).child(button_widget().key("title-1").text("多人")),
        )
        .unwrap();
    let btn = runtime.tree.child_by_key(id, "title-1").unwrap();
    runtime.state.hovered = Some(btn);
    runtime.tree.node_mut(btn).unwrap().state.hovered = true;

    let id2 = runtime
        .reconcile_scene(
            column()
                .key("title-menu")
                .child(button_widget().key("title-0").text("单人模式"))
                .child(button_widget().key("title-1").text("多人模式")),
        )
        .unwrap();

    assert_eq!(id2, id);
    assert_eq!(runtime.state.hovered, Some(btn));
    assert!(runtime.tree.node(btn).unwrap().state.hovered);
    assert_eq!(runtime.tree.node(btn).unwrap().content.text.as_deref(), Some("多人模式"));
}

#[test]
fn reconcile_scene_remounts_when_root_key_changes() {
    let mut runtime = UiRuntime::new();
    let id = runtime.mount_scene(column().key("title-menu").child(label_widget().text("a"))).unwrap();
    let id2 = runtime.reconcile_scene(column().key("pause").child(label_widget().text("b"))).unwrap();
    assert_ne!(id2, id);
    assert!(runtime.tree.node(id).is_none());
}
