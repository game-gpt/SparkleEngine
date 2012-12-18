//! TextField / TextArea 编辑操作。

use crate::{id::WidgetId, node::WidgetKind, tree::WidgetTree};

/// 文本编辑动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEditAction {
    Insert,
    Backspace,
    Delete,
    MoveLeft { select: bool },
    MoveRight { select: bool },
    SelectAll,
    Home { select: bool },
    End { select: bool },
    Copy,
    Cut,
    Paste,
}

/// 向聚焦文本框应用输入与编辑动作。
///
/// `clipboard` 仅在 Copy / Cut / Paste 时使用。
pub fn apply_text_input(
    tree: &mut WidgetTree,
    focused: Option<WidgetId>,
    typed: &str,
    actions: &[TextEditAction],
    mut clipboard: Option<&mut dyn crate::text::Clipboard>,
) -> bool {
    let Some(id) = focused
    else {
        return false;
    };
    let is_field = tree.node(id).map(|n| matches!(n.kind, WidgetKind::TextField | WidgetKind::TextArea)).unwrap_or(false);
    if !is_field {
        return false;
    }

    let Some(node) = tree.node_mut(id)
    else {
        return false;
    };
    let text = node.content.text.get_or_insert_with(String::new);
    let mut cursor = node.content.cursor.min(text.chars().count());
    let mut anchor = node.content.sel_anchor;
    let mut changed = false;

    for action in actions {
        match *action {
            TextEditAction::Backspace => {
                if let Some(a) = anchor {
                    if a != cursor {
                        delete_range(text, &mut cursor, &mut anchor);
                        changed = true;
                        continue;
                    }
                }
                if cursor > 0 {
                    remove_char_before(text, &mut cursor);
                    anchor = None;
                    changed = true;
                }
            }
            TextEditAction::Delete => {
                if let Some(a) = anchor {
                    if a != cursor {
                        delete_range(text, &mut cursor, &mut anchor);
                        changed = true;
                        continue;
                    }
                }
                if cursor < text.chars().count() {
                    remove_char_at(text, cursor);
                    anchor = None;
                    changed = true;
                }
            }
            TextEditAction::MoveLeft { select } => {
                if select && anchor.is_none() {
                    anchor = Some(cursor);
                }
                if !select {
                    if let Some(a) = anchor {
                        cursor = a.min(cursor);
                        anchor = None;
                        continue;
                    }
                }
                cursor = cursor.saturating_sub(1);
                if !select {
                    anchor = None;
                }
            }
            TextEditAction::MoveRight { select } => {
                if select && anchor.is_none() {
                    anchor = Some(cursor);
                }
                if !select {
                    if let Some(a) = anchor {
                        cursor = a.max(cursor);
                        anchor = None;
                        continue;
                    }
                }
                let len = text.chars().count();
                if cursor < len {
                    cursor += 1;
                }
                if !select {
                    anchor = None;
                }
            }
            TextEditAction::Home { select } => {
                if select && anchor.is_none() {
                    anchor = Some(cursor);
                }
                cursor = 0;
                if !select {
                    anchor = None;
                }
            }
            TextEditAction::End { select } => {
                if select && anchor.is_none() {
                    anchor = Some(cursor);
                }
                cursor = text.chars().count();
                if !select {
                    anchor = None;
                }
            }
            TextEditAction::SelectAll => {
                anchor = Some(0);
                cursor = text.chars().count();
            }
            TextEditAction::Copy => {
                if let Some(clip) = clipboard.as_deref_mut() {
                    if let Some(selected) = selected_slice(text, cursor, anchor) {
                        clip.set_text(selected);
                    }
                }
            }
            TextEditAction::Cut => {
                if let Some(clip) = clipboard.as_deref_mut() {
                    if let Some(selected) = selected_slice(text, cursor, anchor) {
                        clip.set_text(selected);
                        delete_range(text, &mut cursor, &mut anchor);
                        changed = true;
                    }
                }
            }
            TextEditAction::Paste => {
                if let Some(clip) = clipboard.as_deref_mut() {
                    if let Some(paste) = clip.get_text() {
                        if !paste.is_empty() {
                            if let Some(a) = anchor {
                                if a != cursor {
                                    delete_range(text, &mut cursor, &mut anchor);
                                }
                            }
                            insert_at(text, cursor, &paste);
                            cursor += paste.chars().count();
                            anchor = None;
                            changed = true;
                        }
                    }
                }
            }
            TextEditAction::Insert => {}
        }
    }

    if !typed.is_empty() {
        if let Some(a) = anchor {
            if a != cursor {
                delete_range(text, &mut cursor, &mut anchor);
            }
        }
        insert_at(text, cursor, typed);
        cursor += typed.chars().count();
        anchor = None;
        changed = true;
    }

    node.content.cursor = cursor;
    node.content.sel_anchor = anchor;
    changed
}

fn selected_slice<'a>(text: &'a str, cursor: usize, anchor: Option<usize>) -> Option<&'a str> {
    let a = anchor?;
    if a == cursor {
        return None;
    }
    let (lo, hi) = if a < cursor { (a, cursor) } else { (cursor, a) };
    let start = byte_index(text, lo);
    let end = byte_index(text, hi);
    Some(&text[start..end])
}

fn byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices().nth(char_index).map(|(i, _)| i).unwrap_or(text.len())
}

fn insert_at(text: &mut String, cursor: usize, typed: &str) {
    let i = byte_index(text, cursor);
    text.insert_str(i, typed);
}

fn remove_char_before(text: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let end = byte_index(text, *cursor);
    let start = byte_index(text, *cursor - 1);
    text.replace_range(start..end, "");
    *cursor -= 1;
}

fn remove_char_at(text: &mut String, cursor: usize) {
    let start = byte_index(text, cursor);
    let end = byte_index(text, cursor + 1);
    if start < end {
        text.replace_range(start..end, "");
    }
}

fn delete_range(text: &mut String, cursor: &mut usize, anchor: &mut Option<usize>) {
    let Some(a) = *anchor
    else {
        return;
    };
    let (lo, hi) = if a <= *cursor { (a, *cursor) } else { (*cursor, a) };
    let start = byte_index(text, lo);
    let end = byte_index(text, hi);
    text.replace_range(start..end, "");
    *cursor = lo;
    *anchor = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::text_field_widget;

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
        use crate::text::{Clipboard, MemoryClipboard};

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
}
