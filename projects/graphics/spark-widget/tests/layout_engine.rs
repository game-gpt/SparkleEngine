//! 自 `src/layout/engine.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_types::Vec2;
use spark_widget::{
    layout::{LayoutSpec, Size},
    text::EstimateMeasurer,
    widgets::{button_widget, column, label_widget, row},
};

#[test]
fn column_stacks_children_vertically() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let menu = column()
        .layout(LayoutSpec::vertical().with_gap(10.0))
        .child(label_widget().text("A").layout(LayoutSpec::default().with_height(Size::Px(20.0))))
        .child(label_widget().text("B").layout(LayoutSpec::default().with_height(Size::Px(30.0))))
        .mount(&mut tree, root)
        .unwrap();

    run_layout(&mut tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

    let a = tree.node(menu).unwrap().children[0];
    let b = tree.node(menu).unwrap().children[1];
    let a_rect = tree.node(a).unwrap().computed.rect;
    let b_rect = tree.node(b).unwrap().computed.rect;
    assert!((a_rect.h - 20.0).abs() < 0.01);
    assert!((b_rect.h - 30.0).abs() < 0.01);
    assert!((b_rect.y - (a_rect.y + a_rect.h + 10.0)).abs() < 0.01);
}

#[test]
fn row_flex_grow_fills_space() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let bar = row()
        .layout(LayoutSpec::horizontal().with_width(Size::Fill).with_height(Size::Px(40.0)))
        .child(button_widget().text("L").layout(LayoutSpec {
            width: Size::Px(40.0),
            height: Size::Px(40.0),
            flex_grow: 0.0,
            ..LayoutSpec::horizontal()
        }))
        .child(button_widget().text("Grow").layout(LayoutSpec {
            width: Size::Px(10.0),
            height: Size::Px(40.0),
            flex_grow: 1.0,
            flex_shrink: 0.0,
            ..LayoutSpec::horizontal()
        }))
        .mount(&mut tree, root)
        .unwrap();

    run_layout(&mut tree, Vec2::new(200.0, 80.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let grow = tree.node(bar).unwrap().children[1];
    let rect = tree.node(grow).unwrap().computed.rect;
    assert!(rect.w > 100.0, "grow child should expand, got {}", rect.w);
}

#[test]
fn root_fills_screen() {
    let mut tree = WidgetTree::new();
    run_layout(&mut tree, Vec2::new(640.0, 360.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let rect = tree.node(tree.root()).unwrap().computed.rect;
    assert_eq!(rect.w, 640.0);
    assert_eq!(rect.h, 360.0);
}

#[test]
fn grid_places_children_in_columns() {
    use spark_widget::widgets::grid;

    let mut tree = WidgetTree::new();
    let root = tree.root();
    let g = grid(2)
        .layout(LayoutSpec::grid(2).with_gap(10.0).with_width(Size::Px(210.0)))
        .child(label_widget().text("A").layout(LayoutSpec::default().with_height(Size::Px(20.0))))
        .child(label_widget().text("B").layout(LayoutSpec::default().with_height(Size::Px(20.0))))
        .child(label_widget().text("C").layout(LayoutSpec::default().with_height(Size::Px(30.0))))
        .mount(&mut tree, root)
        .unwrap();
    run_layout(&mut tree, Vec2::new(400.0, 400.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let kids = &tree.node(g).unwrap().children;
    let a = tree.node(kids[0]).unwrap().computed.rect;
    let b = tree.node(kids[1]).unwrap().computed.rect;
    let c = tree.node(kids[2]).unwrap().computed.rect;
    assert!(b.x > a.x + a.w, "B should be in the next column, a={a:?} b={b:?}");
    assert!((b.y - a.y).abs() < 1.0, "A and B should share a row");
    assert!(c.y > a.y + a.h - 0.1, "C should be on the next row, a={a:?} c={c:?}");
    assert!((c.x - a.x).abs() < 1.0, "C should align to first column");
}

#[test]
fn scroll_view_offsets_children_and_clips() {
    use spark_widget::widgets::scroll_view;

    let mut tree = WidgetTree::new();
    let root = tree.root();
    let scroll = scroll_view()
        .layout(LayoutSpec { width: Size::Px(100.0), height: Size::Px(80.0), ..LayoutSpec::vertical() })
        .child(
            column()
                .layout(LayoutSpec::vertical().with_gap(0.0))
                .child(label_widget().text("top").layout(LayoutSpec::default().with_height(Size::Px(60.0))))
                .child(label_widget().text("bottom").layout(LayoutSpec::default().with_height(Size::Px(60.0)))),
        )
        .mount(&mut tree, root)
        .unwrap();

    run_layout(&mut tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    if let Some(node) = tree.node_mut(scroll) {
        node.scroll.offset.y = 40.0;
    }
    run_layout(&mut tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

    let content = tree.node(scroll).unwrap().children[0];
    let top = tree.node(content).unwrap().children[0];
    let top_rect = tree.node(top).unwrap().computed.rect;
    let clip = tree.node(scroll).unwrap().computed.clip_rect.unwrap();
    let offset_y = tree.node(scroll).unwrap().scroll.offset.y;
    assert!((offset_y - 40.0).abs() < 0.01, "scroll offset should remain 40, got {offset_y}");
    assert!((top_rect.y - (clip.y - offset_y)).abs() < 1.0, "top_rect.y={} clip.y={} offset={}", top_rect.y, clip.y, offset_y);
    assert!(
        tree.node(scroll).unwrap().scroll.content_size.y >= 120.0,
        "content should exceed viewport, got {}",
        tree.node(scroll).unwrap().scroll.content_size.y
    );
}

#[test]
fn safe_area_insets_root_rect() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    label_widget().text("hi").mount(&mut tree, root).unwrap();
    let metrics = UiMetrics { dpi_scale: 1.0, ui_scale: 1.0, safe_area: Insets { left: 10.0, top: 20.0, right: 10.0, bottom: 30.0 } };
    run_layout(&mut tree, Vec2::new(200.0, 200.0), metrics, &mut EstimateMeasurer);
    let rect = tree.node(root).unwrap().computed.rect;
    assert!((rect.x - 10.0).abs() < 0.01);
    assert!((rect.y - 20.0).abs() < 0.01);
    assert!((rect.w - 180.0).abs() < 0.01);
    assert!((rect.h - 150.0).abs() < 0.01);
}

#[test]
fn content_scale_grows_button_intrinsic() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let id = button_widget().text("Go").mount(&mut tree, root).unwrap();
    run_layout(&mut tree, Vec2::new(400.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let h1 = tree.node(id).unwrap().computed.desired.height;
    run_layout(
        &mut tree,
        Vec2::new(400.0, 200.0),
        UiMetrics { dpi_scale: 2.0, ui_scale: 1.0, safe_area: Insets::default() },
        &mut EstimateMeasurer,
    );
    let h2 = tree.node(id).unwrap().computed.desired.height;
    assert!(h2 > h1 * 1.5, "scaled={h2} base={h1}");
}

#[test]
fn max_content_label_does_not_wrap() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let id = label_widget()
        .text("ABCDEFGHIJKLMNOP")
        .layout(LayoutSpec { width: Size::MaxContent, height: Size::Auto, ..LayoutSpec::default() })
        .mount(&mut tree, root)
        .unwrap();
    run_layout(&mut tree, Vec2::new(80.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let max_c = tree.node(id).unwrap().computed.desired;

    if let Some(n) = tree.node_mut(id) {
        n.layout.width = Size::Auto;
    }
    run_layout(&mut tree, Vec2::new(80.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let auto_c = tree.node(id).unwrap().computed.desired;

    assert!(max_c.width > auto_c.width + 1.0, "max={} auto={}", max_c.width, auto_c.width);
    assert!(max_c.height <= auto_c.height + 0.5, "max_h={} auto_h={}", max_c.height, auto_c.height);
}

#[test]
fn min_content_label_is_narrower_than_max_content() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let id = label_widget()
        .text("HelloWorld")
        .layout(LayoutSpec { width: Size::MinContent, height: Size::Auto, ..LayoutSpec::default() })
        .mount(&mut tree, root)
        .unwrap();
    run_layout(&mut tree, Vec2::new(400.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let min_w = tree.node(id).unwrap().computed.desired.width;

    if let Some(n) = tree.node_mut(id) {
        n.layout.width = Size::MaxContent;
    }
    run_layout(&mut tree, Vec2::new(400.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let max_w = tree.node(id).unwrap().computed.desired.width;
    assert!(min_w + 1.0 < max_w, "min={min_w} max={max_w}");
}
