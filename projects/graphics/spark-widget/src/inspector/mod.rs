//! UI 调试观察（占位）。

use crate::id::WidgetId;
use crate::tree::WidgetTree;

#[derive(Debug, Default)]
pub struct UiInspector {
    pub highlight: Option<WidgetId>,
}

impl UiInspector {
    pub fn tree<'a>(&self, runtime_tree: &'a WidgetTree) -> &'a WidgetTree {
        runtime_tree
    }

    pub fn highlight(&mut self, id: WidgetId) {
        self.highlight = Some(id);
    }

    pub fn clear_highlight(&mut self) {
        self.highlight = None;
    }
}
