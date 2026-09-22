//! 自 `src/binding.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_widget::{id::WidgetId, widgets::label_widget};

struct LabelVm {
    text: String,
    target: Option<WidgetId>,
}

impl UiViewModel for LabelVm {
    fn sync(&mut self, tree: &mut WidgetTree) {
        let Some(id) = self.target else {
            return;
        };
        set_text(tree, id, self.text.clone());
    }
}

#[test]
fn view_model_updates_label_text() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let id = label_widget().text("old").mount(&mut tree, root).unwrap();
    let mut vm = LabelVm {
        text: "new".into(),
        target: Some(id),
    };
    vm.sync(&mut tree);
    assert_eq!(tree.node(id).unwrap().content.text.as_deref(), Some("new"));
}

#[test]
fn bind_helpers_by_id_and_key() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let id = label_widget()
        .key("hud.hp")
        .text("0")
        .mount(&mut tree, root)
        .unwrap();

    assert!(set_text_by_key(&mut tree, root, "hud.hp", "99"));
    assert_eq!(tree.node(id).unwrap().content.text.as_deref(), Some("99"));

    assert!(set_visible_by_key(&mut tree, root, "hud.hp", false));
    assert!(!tree.node(id).unwrap().state.visible);

    assert!(set_selected(&mut tree, id, true));
    assert!(tree.node(id).unwrap().state.selected);

    assert!(set_checked(&mut tree, id, true));
    assert!(tree.node(id).unwrap().content.checked);
    assert!(tree.node(id).unwrap().state.checked);

    assert!(set_value(&mut tree, id, 0.5));
    assert!((tree.node(id).unwrap().content.value - 0.5).abs() < f32::EPSILON);

    assert!(!set_text_by_key(&mut tree, root, "missing", "x"));
}
