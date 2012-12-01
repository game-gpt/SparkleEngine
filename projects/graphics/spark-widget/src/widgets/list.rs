//! ListView 可见行范围与行同步。

use crate::id::WidgetId;
use crate::layout::{Layout, LayoutSpec, Size};
use crate::node::WidgetKind;
use crate::tree::WidgetTree;
use crate::widgets::WidgetBuilder;

/// 计算垂直列表在当前滚动偏移下应实例化的行区间 `[start, end)`。
pub fn visible_row_range(
    scroll_offset_y: f32,
    viewport_height: f32,
    row_height: f32,
    item_count: usize,
    overscan: usize,
) -> (usize, usize) {
    if item_count == 0 || row_height <= f32::EPSILON || viewport_height <= 0.0 {
        return (0, 0);
    }
    let first = (scroll_offset_y / row_height).floor().max(0.0) as usize;
    let visible = ((viewport_height / row_height).ceil() as usize).saturating_add(1);
    let start = first.saturating_sub(overscan);
    let end = (first + visible + overscan).min(item_count);
    (start, end)
}

/// 列表内容总高度。
pub fn content_height(item_count: usize, row_height: f32) -> f32 {
    item_count as f32 * row_height.max(0.0)
}

/// 按当前滚动偏移重建 `ListView` 可见行。
///
/// 结构：`list` → 绝对定位内容板（总高度）→ 仅可见行（`offset_y = index * row_height`）。
pub fn sync_visible_rows<F>(
    tree: &mut WidgetTree,
    list_id: WidgetId,
    item_count: usize,
    row_height: f32,
    overscan: usize,
    mut build_row: F,
) -> Option<(usize, usize)>
where
    F: FnMut(usize) -> WidgetBuilder,
{
    let kind = tree.node(list_id).map(|n| n.kind)?;
    if kind != WidgetKind::ListView {
        return None;
    }
    let scroll = tree.node(list_id).map(|n| n.scroll.clone())?;
    let (start, end) = visible_row_range(
        scroll.offset.y,
        scroll.viewport_size.y.max(1.0),
        row_height,
        item_count,
        overscan,
    );

    tree.clear_children(list_id);
    let total_h = content_height(item_count, row_height);
    let panel = tree.mount(list_id, WidgetKind::Container)?;
    if let Some(node) = tree.node_mut(panel) {
        node.layout = LayoutSpec {
            kind: Layout::Absolute,
            width: Size::Fill,
            height: Size::Px(total_h.max(1.0)),
            ..LayoutSpec::default()
        };
    }

    for index in start..end {
        let row = build_row(index).layout(LayoutSpec {
            kind: Layout::Absolute,
            width: Size::Fill,
            height: Size::Px(row_height),
            offset_x: 0.0,
            offset_y: index as f32 * row_height,
            ..LayoutSpec::default()
        });
        let row_id = row.mount(tree, panel)?;
        if let Some(node) = tree.node_mut(row_id) {
            node.content.value = index as f32;
        }
    }

    if let Some(node) = tree.node_mut(list_id) {
        node.scroll.content_size.y = total_h;
        node.scroll.clamp_offset();
    }
    Some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::UiMetrics;
    use crate::layout::{LayoutSpec, Size, run_layout};
    use crate::text::EstimateMeasurer;
    use crate::widgets::{label_widget, list_view};
    use spark_core::Vec2;

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
            .layout(LayoutSpec {
                width: Size::Px(100.0),
                height: Size::Px(80.0),
                ..LayoutSpec::vertical()
            })
            .mount(&mut tree, root)
            .unwrap();

        run_layout(
            &mut tree,
            Vec2::new(200.0, 200.0),
            UiMetrics::new(1.0),
            &mut EstimateMeasurer,
        );
        let range = sync_visible_rows(&mut tree, list, 100, 20.0, 1, |i| {
            label_widget().text(format!("row-{i}"))
        })
        .unwrap();
        assert_eq!(range, (0, 6)); // viewport 80 → ~4+1 rows + overscan 1 → 0..6

        let panel = tree.node(list).unwrap().children[0];
        assert_eq!(tree.node(panel).unwrap().children.len(), 6);

        if let Some(node) = tree.node_mut(list) {
            node.scroll.offset.y = 200.0;
        }
        run_layout(
            &mut tree,
            Vec2::new(200.0, 200.0),
            UiMetrics::new(1.0),
            &mut EstimateMeasurer,
        );
        let range = sync_visible_rows(&mut tree, list, 100, 20.0, 0, |i| {
            label_widget().text(format!("row-{i}"))
        })
        .unwrap();
        assert_eq!(range.0, 10);
        assert!(range.1 > range.0);
        assert!(tree.node(panel).is_none() || tree.node(list).unwrap().children.len() == 1);
        let panel = tree.node(list).unwrap().children[0];
        assert_eq!(tree.node(panel).unwrap().children.len(), range.1 - range.0);
    }
}
