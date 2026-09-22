//! ListView 可见行范围与行同步。

use crate::{
    id::WidgetId,
    layout::{Layout, LayoutSpec, Size},
    node::WidgetKind,
    tree::WidgetTree,
    widgets::WidgetBuilder,
};

/// 计算垂直列表在当前滚动偏移下应实例化的行区间 `[start, end)`。
pub fn visible_row_range(scroll_offset_y: f32, viewport_height: f32, row_height: f32, item_count: usize, overscan: usize) -> (usize, usize) {
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
    let (start, end) = visible_row_range(scroll.offset.y, scroll.viewport_size.y.max(1.0), row_height, item_count, overscan);

    tree.clear_children(list_id);
    let total_h = content_height(item_count, row_height);
    let panel = tree.mount(list_id, WidgetKind::Container)?;
    if let Some(node) = tree.node_mut(panel) {
        node.layout = LayoutSpec { kind: Layout::Absolute, width: Size::Fill, height: Size::Px(total_h.max(1.0)), ..LayoutSpec::default() };
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
