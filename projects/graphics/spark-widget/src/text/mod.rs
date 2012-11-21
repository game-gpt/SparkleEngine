//! 文本测量、排版与基础编辑。

use spark_core::{Color, Vec2};

use crate::id::WidgetId;
use crate::node::WidgetKind;
use crate::tree::WidgetTree;

/// 文本样式占位。
#[derive(Debug, Clone)]
pub struct TextStyle {
    pub size: f32,
    pub color: Color,
    pub line_height: f32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            size: 16.0,
            color: Color::rgb(0.92, 0.94, 0.96),
            line_height: 1.25,
        }
    }
}

/// 文本布局结果占位。
#[derive(Debug, Clone, Default)]
pub struct TextLayout {
    pub size: Vec2,
    pub baseline: f32,
    pub line_count: usize,
}

/// 测量纯文本（占位：按字符数粗估，后续接 spark-font）。
pub fn measure_plain(text: &str, style: &TextStyle, max_width: Option<f32>) -> TextLayout {
    let _ = max_width;
    let w = text.chars().count() as f32 * style.size * 0.55;
    TextLayout {
        size: Vec2::new(w, style.size * style.line_height),
        baseline: style.size,
        line_count: 1,
    }
}

/// 向聚焦的文本控件追加本帧字符，并处理退格。
pub fn apply_text_input(tree: &mut WidgetTree, focused: Option<WidgetId>, typed: &str, backspace: bool) -> bool {
    let Some(id) = focused else {
        return false;
    };
    let is_field = tree
        .node(id)
        .map(|n| matches!(n.kind, WidgetKind::TextField | WidgetKind::TextArea))
        .unwrap_or(false);
    if !is_field {
        return false;
    }

    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    let mut changed = false;
    if backspace {
        let text = node.content.text.get_or_insert_with(String::new);
        if text.pop().is_some() {
            changed = true;
        }
    }
    if !typed.is_empty() {
        let text = node.content.text.get_or_insert_with(String::new);
        text.push_str(typed);
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::text_field_widget;

    #[test]
    fn apply_text_and_backspace() {
        let mut tree = WidgetTree::new();
        let root = tree.root();
        let id = text_field_widget()
            .text("ab")
            .mount(&mut tree, root)
            .unwrap();
        assert!(apply_text_input(&mut tree, Some(id), "c", false));
        assert_eq!(tree.node(id).unwrap().content.text.as_deref(), Some("abc"));
        assert!(apply_text_input(&mut tree, Some(id), "", true));
        assert_eq!(tree.node(id).unwrap().content.text.as_deref(), Some("ab"));
    }
}
