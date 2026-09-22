//! 自 `src/accessibility/mod.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_widget::widgets::{button_widget, checkbox_widget, column};

#[test]
fn rebuild_includes_interactive_widgets() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    column().child(button_widget().text("Go")).child(checkbox_widget().text("On").checked(true)).mount(&mut tree, root).unwrap();
    let mut a11y = AccessibilityTree::default();
    a11y.rebuild_from(&tree);
    assert!(a11y.nodes().iter().any(|n| n.role == Role::Button && n.label == "Go"));
    assert!(a11y.nodes().iter().any(|n| n.role == Role::Checkbox && n.checked == Some(true)));
}
