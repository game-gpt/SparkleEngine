//! 自 `src/event/bubble.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_widget::widgets::{button_widget, column};

#[test]
fn bubble_path_lists_ancestors() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let col = column().child(button_widget().text("x")).mount(&mut tree, root).unwrap();
    let btn = tree.node(col).unwrap().children[0];
    let path = bubble_path(&tree, btn);
    assert_eq!(path[0], btn);
    assert!(path.contains(&col));
    assert!(path.contains(&root));
}

#[test]
fn stop_halts_further_ancestors() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let col = column().child(button_widget().text("x")).mount(&mut tree, root).unwrap();
    let btn = tree.node(col).unwrap().children[0];
    let mut seen = Vec::new();
    let resp = bubble_from(&tree, btn, |id| {
        seen.push(id);
        if id == btn { EventResponse::stop() } else { EventResponse::default() }
    });
    assert!(resp.stop_propagation);
    assert_eq!(seen, vec![btn]);
}
