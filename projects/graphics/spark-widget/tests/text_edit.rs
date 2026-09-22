//! 自 `src/text/edit.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_widget::widgets::text_field_widget;

#[test]
fn insert_and_backspace_at_cursor() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let id = text_field_widget().text("ab").mount(&mut tree, root).unwrap();
    if let Some(n) = tree.node_mut(id) {
        n.content.cursor = 2;
    }
    assert!(apply_text_input(&mut tree, Some(id), "c", &[], None,));
    assert_eq!(tree.node(id).unwrap().content.text.as_deref(), Some("abc"));
    assert!(apply_text_input(&mut tree, Some(id), "", &[TextEditAction::Backspace], None,));
    assert_eq!(tree.node(id).unwrap().content.text.as_deref(), Some("ab"));
}

#[test]
fn selection_delete_replaces_range() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let id = text_field_widget().text("hello").mount(&mut tree, root).unwrap();
    if let Some(n) = tree.node_mut(id) {
        n.content.sel_anchor = Some(1);
        n.content.cursor = 4;
    }
    assert!(apply_text_input(&mut tree, Some(id), "i", &[], None));
    assert_eq!(tree.node(id).unwrap().content.text.as_deref(), Some("hio"));
}

#[test]
fn copy_cut_paste_via_clipboard() {
    use spark_widget::text::{Clipboard, MemoryClipboard};

    let mut tree = WidgetTree::new();
    let root = tree.root();
    let id = text_field_widget().text("abcd").mount(&mut tree, root).unwrap();
    if let Some(n) = tree.node_mut(id) {
        n.content.sel_anchor = Some(1);
        n.content.cursor = 3;
    }
    let mut clip = MemoryClipboard::new();
    assert!(!apply_text_input(&mut tree, Some(id), "", &[TextEditAction::Copy], Some(&mut clip),));
    assert_eq!(clip.get_text().as_deref(), Some("bc"));
    assert!(apply_text_input(&mut tree, Some(id), "", &[TextEditAction::Cut], Some(&mut clip),));
    assert_eq!(tree.node(id).unwrap().content.text.as_deref(), Some("ad"));
    if let Some(n) = tree.node_mut(id) {
        n.content.cursor = 1;
        n.content.sel_anchor = None;
    }
    assert!(apply_text_input(&mut tree, Some(id), "", &[TextEditAction::Paste], Some(&mut clip),));
    assert_eq!(tree.node(id).unwrap().content.text.as_deref(), Some("abcd"));
}
