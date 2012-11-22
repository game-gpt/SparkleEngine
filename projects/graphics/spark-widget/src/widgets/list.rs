//! ListView 可见行范围（虚拟化骨架）。

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_range_includes_overscan() {
        let (start, end) = visible_row_range(100.0, 80.0, 20.0, 50, 1);
        // offset 100 → row 5, viewport ~4 rows → 5..10 plus overscan → 4..11
        assert_eq!(start, 4);
        assert_eq!(end, 11);
    }

    #[test]
    fn empty_list_yields_empty_range() {
        assert_eq!(visible_row_range(0.0, 100.0, 20.0, 0, 2), (0, 0));
    }
}
