//! 自 `src/widgets/list.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_types::Vec2;
use spark_widget::{
    layout::{LayoutSpec, Size, UiMetrics, run_layout},
    text::EstimateMeasurer,
    widgets::{label_widget, list_view},
};

#[test]
fn visible_range_includes_overscan() {
    let (start, end) = visible_row_range(100.0, 80.0, 20.0, 50, 1);
    assert_eq!(start, 4);
    assert_eq!(end, 11);
}

#[test]
fn empty_list_yields_empty_range() {
    assert_eq!(visible_row_range(0.0, 100.0, 20.0, 0, 2), (0, 0));
}

#[test]
fn sync_only_mounts_visible_rows() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let list = list_view()
        .layout(LayoutSpec { width: Size::Px(100.0), height: Size::Px(80.0), ..LayoutSpec::vertical() })
        .mount(&mut tree, root)
        .unwrap();

    run_layout(&mut tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let range = sync_visible_rows(&mut tree, list, 100, 20.0, 1, |i| label_widget().text(format!("row-{i}"))).unwrap();
    assert_eq!(range, (0, 6)); // viewport 80 → ~4+1 rows + overscan 1 → 0..6

    let panel = tree.node(list).unwrap().children[0];
    assert_eq!(tree.node(panel).unwrap().children.len(), 6);

    if let Some(node) = tree.node_mut(list) {
        node.scroll.offset.y = 200.0;
    }
    run_layout(&mut tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let range = sync_visible_rows(&mut tree, list, 100, 20.0, 0, |i| label_widget().text(format!("row-{i}"))).unwrap();
    assert_eq!(range.0, 10);
    assert!(range.1 > range.0);
    assert!(tree.node(panel).is_none() || tree.node(list).unwrap().children.len() == 1);
    let panel = tree.node(list).unwrap().children[0];
    assert_eq!(tree.node(panel).unwrap().children.len(), range.1 - range.0);
}
