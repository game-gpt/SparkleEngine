//! 自 `src/binding.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_widget::{id::WidgetId, widgets::label_widget};

struct LabelVm {
    text: String,
    target: Option<WidgetId>,
}

impl UiViewModel for LabelVm {
    fn sync(&mut self, tree: &mut WidgetTree) {
        let Some(id) = self.target
        else {
            return;
        };
        if let Some(node) = tree.node_mut(id) {
            node.content.text = Some(self.text.clone());
        }
    }
}

#[test]
fn view_model_updates_label_text() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let id = label_widget().text("old").mount(&mut tree, root).unwrap();
    let mut vm = LabelVm { text: "new".into(), target: Some(id) };
    vm.sync(&mut tree);
    assert_eq!(tree.node(id).unwrap().content.text.as_deref(), Some("new"));
}
