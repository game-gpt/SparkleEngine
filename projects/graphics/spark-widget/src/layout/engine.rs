//! measure / arrange 引擎。

use spark_core::{Rect, Vec2};

use crate::id::WidgetId;
use crate::node::{WidgetKind, WidgetNode};
use crate::text::{self, TextStyle};
use crate::tree::WidgetTree;

use super::{
    Align, Constraints, FlexDirection, Insets, Justify, Layout, LayoutSpec, Size, Size2,
};

/// 对整棵树跑 measure + arrange。根节点填满屏幕。
pub fn run_layout(tree: &mut WidgetTree, screen: Vec2, _dpi_scale: f32) {
    let root = tree.root();
    let screen_size = Size2::new(screen.x.max(0.0), screen.y.max(0.0));
    let constraints = Constraints::tight(screen_size);
    let _ = measure(tree, root, constraints);
    arrange(
        tree,
        root,
        Rect::new(0.0, 0.0, screen_size.width, screen_size.height),
    );
}

fn measure(tree: &mut WidgetTree, id: WidgetId, constraints: Constraints) -> Size2 {
    let Some(node) = tree.node(id).cloned() else {
        return Size2::default();
    };
    if !node.state.visible {
        let zero = Size2::default();
        if let Some(n) = tree.node_mut(id) {
            n.computed.desired = zero;
        }
        return zero;
    }

    let margin = node.layout.margin;
    let inner_constraints = apply_spec_limits(
        constraints.deflate(margin),
        &node.layout,
    );

    let content = measure_content(tree, &node, inner_constraints.deflate(node.layout.padding));
    let padded = Size2::new(
        content.width + node.layout.padding.horizontal(),
        content.height + node.layout.padding.vertical(),
    );
    let sized = resolve_axis_sizes(padded, &node.layout, inner_constraints);
    let with_margin = Size2::new(
        sized.width + margin.horizontal(),
        sized.height + margin.vertical(),
    );
    let desired = with_margin.clamp(constraints);

    if let Some(n) = tree.node_mut(id) {
        n.computed.desired = desired;
    }
    desired
}

fn measure_content(tree: &mut WidgetTree, node: &WidgetNode, constraints: Constraints) -> Size2 {
    match node.layout.kind {
        Layout::Flex | Layout::Grid => measure_flex(tree, node, constraints),
        Layout::Overlay | Layout::Stack | Layout::Anchor => measure_overlay(tree, node, constraints),
        Layout::Absolute => measure_absolute(tree, node, constraints),
    }
}

fn measure_flex(tree: &mut WidgetTree, node: &WidgetNode, constraints: Constraints) -> Size2 {
    let children: Vec<WidgetId> = visible_children(tree, node);
    if children.is_empty() {
        return intrinsic_leaf(node, constraints);
    }

    let gap = node.layout.gap;
    let gap_total = if children.len() > 1 {
        gap * (children.len() as f32 - 1.0)
    } else {
        0.0
    };

    let vertical = node.layout.direction == FlexDirection::Column;
    let mut main = 0.0_f32;
    let mut cross = 0.0_f32;

    for child in &children {
        let child_constraints = if vertical {
            Constraints::loose(Size2::new(constraints.max.width, f32::INFINITY))
                .with_max_width(constraints.max.width)
        } else {
            Constraints::loose(Size2::new(f32::INFINITY, constraints.max.height))
                .with_max_height(constraints.max.height)
        };
        let size = measure(tree, *child, child_constraints);
        if vertical {
            main += size.height;
            cross = cross.max(size.width);
        } else {
            main += size.width;
            cross = cross.max(size.height);
        }
    }
    main += gap_total;

    if vertical {
        Size2::new(cross.min(constraints.max.width), main.min(constraints.max.height))
    } else {
        Size2::new(main.min(constraints.max.width), cross.min(constraints.max.height))
    }
}

fn measure_overlay(tree: &mut WidgetTree, node: &WidgetNode, constraints: Constraints) -> Size2 {
    let children = visible_children(tree, node);
    if children.is_empty() {
        return intrinsic_leaf(node, constraints);
    }
    let mut size = Size2::default();
    for child in children {
        let child_size = measure(tree, child, constraints);
        size.width = size.width.max(child_size.width);
        size.height = size.height.max(child_size.height);
    }
    size.clamp(constraints)
}

fn measure_absolute(tree: &mut WidgetTree, node: &WidgetNode, constraints: Constraints) -> Size2 {
    let children = visible_children(tree, node);
    let mut size = intrinsic_leaf(node, constraints);
    for child in children {
        let child_size = measure(tree, child, Constraints::loose(constraints.max));
        if let Some(child_node) = tree.node(child) {
            let right = child_node.layout.offset_x + child_size.width;
            let bottom = child_node.layout.offset_y + child_size.height;
            size.width = size.width.max(right);
            size.height = size.height.max(bottom);
        }
    }
    size.clamp(constraints)
}

fn intrinsic_leaf(node: &WidgetNode, constraints: Constraints) -> Size2 {
    let text = node.content.text.as_deref().unwrap_or("");
    let style = TextStyle {
        size: match node.kind {
            WidgetKind::Label => 16.0,
            WidgetKind::Button => 16.0,
            _ => 14.0,
        },
        ..TextStyle::default()
    };
    let measured = if text.is_empty() {
        Size2::default()
    } else {
        let layout = text::measure_plain(text, &style, Some(constraints.max.width));
        Size2::new(layout.size.x, layout.size.y)
    };

    match node.kind {
        WidgetKind::Separator => {
            if node.layout.direction == FlexDirection::Row {
                Size2::new(constraints.max.width.min(1.0), 1.0)
            } else {
                Size2::new(1.0, constraints.max.height.min(1.0))
            }
        }
        WidgetKind::Spacer => Size2::default(),
        WidgetKind::Button => Size2::new(
            (measured.width + 24.0).max(48.0),
            (measured.height + 12.0).max(32.0),
        ),
        WidgetKind::Label => measured,
        _ => measured,
    }
}

fn resolve_axis_sizes(content: Size2, spec: &LayoutSpec, constraints: Constraints) -> Size2 {
    let width = resolve_size(spec.width, content.width, constraints.max.width, constraints.min.width);
    let height = resolve_size(
        spec.height,
        content.height,
        constraints.max.height,
        constraints.min.height,
    );
    let mut size = Size2::new(width, height);
    if let Some(min_w) = spec.min_width {
        size.width = size.width.max(min_w);
    }
    if let Some(min_h) = spec.min_height {
        size.height = size.height.max(min_h);
    }
    if let Some(max_w) = spec.max_width {
        size.width = size.width.min(max_w);
    }
    if let Some(max_h) = spec.max_height {
        size.height = size.height.min(max_h);
    }
    size.clamp(constraints)
}

fn resolve_size(spec: Size, content: f32, max: f32, min: f32) -> f32 {
    let value = match spec {
        // Fill 在 measure 阶段不抢无限空间，arrange 时再伸展。
        Size::Auto | Size::MinContent | Size::MaxContent | Size::Fill => content,
        Size::Px(v) => v,
        Size::Percent(p) => {
            if max.is_finite() {
                max * (p / 100.0)
            } else {
                content
            }
        }
    };
    let upper = if max.is_finite() { max } else { value.max(min) };
    value.clamp(min, upper)
}

fn apply_spec_limits(mut constraints: Constraints, spec: &LayoutSpec) -> Constraints {
    if let Some(max_w) = spec.max_width {
        constraints = constraints.with_max_width(max_w);
    }
    if let Some(max_h) = spec.max_height {
        constraints = constraints.with_max_height(max_h);
    }
    if let Some(min_w) = spec.min_width {
        constraints.min.width = constraints.min.width.max(min_w).min(constraints.max.width);
    }
    if let Some(min_h) = spec.min_height {
        constraints.min.height = constraints.min.height.max(min_h).min(constraints.max.height);
    }
    constraints
}

fn arrange(tree: &mut WidgetTree, id: WidgetId, rect: Rect) {
    let Some(node) = tree.node(id).cloned() else {
        return;
    };
    if !node.state.visible {
        return;
    }

    let margin = node.layout.margin;
    let border = Rect::new(
        rect.x + margin.left,
        rect.y + margin.top,
        (rect.w - margin.horizontal()).max(0.0),
        (rect.h - margin.vertical()).max(0.0),
    );
    let content = deflate_rect(border, node.layout.padding);

    if let Some(n) = tree.node_mut(id) {
        n.computed.rect = border;
        n.computed.content_rect = content;
        n.computed.clip_rect = Some(border);
    }

    match node.layout.kind {
        Layout::Flex | Layout::Grid => arrange_flex(tree, &node, content),
        Layout::Overlay | Layout::Stack | Layout::Anchor => arrange_overlay(tree, &node, content),
        Layout::Absolute => arrange_absolute(tree, &node, content),
    }
}

fn arrange_flex(tree: &mut WidgetTree, node: &WidgetNode, content: Rect) {
    let children = visible_children(tree, node);
    if children.is_empty() {
        return;
    }

    let vertical = node.layout.direction == FlexDirection::Column;
    let gap = node.layout.gap;
    let gap_total = if children.len() > 1 {
        gap * (children.len() as f32 - 1.0)
    } else {
        0.0
    };

    let mut bases = Vec::with_capacity(children.len());
    let mut total_main = 0.0_f32;
    let mut total_grow = 0.0_f32;
    let mut total_shrink = 0.0_f32;

    for child in &children {
        let desired = tree
            .node(*child)
            .map(|n| n.computed.desired)
            .unwrap_or_default();
        let (grow, shrink, main_fill) = tree
            .node(*child)
            .map(|n| {
                let main_fill = if vertical {
                    matches!(n.layout.height, Size::Fill)
                } else {
                    matches!(n.layout.width, Size::Fill)
                };
                let grow = if main_fill {
                    n.layout.flex_grow.max(1.0)
                } else {
                    n.layout.flex_grow.max(0.0)
                };
                (grow, n.layout.flex_shrink.max(0.0), main_fill)
            })
            .unwrap_or((0.0, 0.0, false));
        let base_main = if main_fill {
            0.0
        } else if vertical {
            desired.height
        } else {
            desired.width
        };
        bases.push((desired, grow, shrink, base_main));
        total_main += base_main;
        total_grow += grow;
        total_shrink += shrink;
    }

    let available = if vertical { content.h } else { content.w };
    let free = available - total_main - gap_total;
    let mut mains = Vec::with_capacity(children.len());
    for (_, grow, shrink, base) in &bases {
        let mut main = *base;
        if free > 0.0 && total_grow > 0.0 && *grow > 0.0 {
            main += free * (*grow / total_grow);
        } else if free < 0.0 && total_shrink > 0.0 && *shrink > 0.0 {
            main += free * (*shrink / total_shrink);
            main = main.max(0.0);
        }
        mains.push(main);
    }

    let used_main: f32 = mains.iter().sum::<f32>() + gap_total;
    let justify_offset = match node.layout.justify {
        Justify::Start => 0.0,
        Justify::Center => ((available - used_main) * 0.5).max(0.0),
        Justify::End => (available - used_main).max(0.0),
        Justify::SpaceBetween => 0.0,
    };
    let between = if node.layout.justify == Justify::SpaceBetween && children.len() > 1 {
        ((available - (used_main - gap_total)) / (children.len() as f32 - 1.0)).max(0.0)
    } else {
        gap
    };

    let mut cursor = if vertical {
        content.y + justify_offset
    } else {
        content.x + justify_offset
    };

    for (index, child) in children.iter().enumerate() {
        let desired = bases[index].0;
        let main = mains[index];
        let child_layout = tree.node(*child).map(|n| n.layout.clone());
        let (child_w, child_h, x, y) = if vertical {
            let width = match (
                child_layout.as_ref().map(|l| l.width),
                node.layout.align,
            ) {
                (Some(Size::Fill), _) | (_, Align::Stretch) => content.w,
                _ => desired.width.min(content.w),
            };
            let x = align_cross(content.x, content.w, width, node.layout.align);
            (width, main, x, cursor)
        } else {
            let height = match (
                child_layout.as_ref().map(|l| l.height),
                node.layout.align,
            ) {
                (Some(Size::Fill), _) | (_, Align::Stretch) => content.h,
                _ => desired.height.min(content.h),
            };
            let y = align_cross(content.y, content.h, height, node.layout.align);
            (main, height, cursor, y)
        };
        arrange(tree, *child, Rect::new(x, y, child_w, child_h));
        cursor += main + between;
    }
}

fn arrange_overlay(tree: &mut WidgetTree, node: &WidgetNode, content: Rect) {
    for child in visible_children(tree, node) {
        let desired = tree
            .node(child)
            .map(|n| n.computed.desired)
            .unwrap_or_default();
        let align = tree
            .node(child)
            .map(|n| n.layout.align)
            .unwrap_or_default();
        let width = match align {
            Align::Stretch => content.w,
            _ => desired.width.min(content.w),
        };
        let height = match align {
            Align::Stretch => content.h,
            _ => desired.height.min(content.h),
        };
        let x = align_cross(content.x, content.w, width, align);
        let y = align_cross(content.y, content.h, height, align);
        arrange(tree, child, Rect::new(x, y, width, height));
    }
}

fn arrange_absolute(tree: &mut WidgetTree, node: &WidgetNode, content: Rect) {
    for child in visible_children(tree, node) {
        let (desired, offset_x, offset_y, width_spec, height_spec) = {
            let Some(child_node) = tree.node(child) else {
                continue;
            };
            (
                child_node.computed.desired,
                child_node.layout.offset_x,
                child_node.layout.offset_y,
                child_node.layout.width,
                child_node.layout.height,
            )
        };
        let width = match width_spec {
            Size::Fill => (content.w - offset_x).max(0.0),
            _ => desired.width,
        };
        let height = match height_spec {
            Size::Fill => (content.h - offset_y).max(0.0),
            _ => desired.height,
        };
        arrange(
            tree,
            child,
            Rect::new(content.x + offset_x, content.y + offset_y, width, height),
        );
    }
}

fn align_cross(origin: f32, span: f32, child: f32, align: Align) -> f32 {
    match align {
        Align::Start | Align::Stretch => origin,
        Align::Center => origin + ((span - child) * 0.5).max(0.0),
        Align::End => origin + (span - child).max(0.0),
    }
}

fn deflate_rect(rect: Rect, padding: Insets) -> Rect {
    Rect::new(
        rect.x + padding.left,
        rect.y + padding.top,
        (rect.w - padding.horizontal()).max(0.0),
        (rect.h - padding.vertical()).max(0.0),
    )
}

fn visible_children(tree: &WidgetTree, node: &WidgetNode) -> Vec<WidgetId> {
    node.children
        .iter()
        .copied()
        .filter(|id| tree.node(*id).map(|n| n.state.visible).unwrap_or(false))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{LayoutSpec, Size};
    use crate::widgets::{button_widget, column, label_widget, row};

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

        run_layout(&mut tree, Vec2::new(200.0, 200.0), 1.0);

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
            .layout(
                LayoutSpec::horizontal()
                    .with_width(Size::Fill)
                    .with_height(Size::Px(40.0)),
            )
            .child(
                button_widget()
                    .text("L")
                    .layout(LayoutSpec {
                        width: Size::Px(40.0),
                        height: Size::Px(40.0),
                        flex_grow: 0.0,
                        ..LayoutSpec::horizontal()
                    }),
            )
            .child(
                button_widget()
                    .text("Grow")
                    .layout(LayoutSpec {
                        width: Size::Px(10.0),
                        height: Size::Px(40.0),
                        flex_grow: 1.0,
                        flex_shrink: 0.0,
                        ..LayoutSpec::horizontal()
                    }),
            )
            .mount(&mut tree, root)
            .unwrap();

        run_layout(&mut tree, Vec2::new(200.0, 80.0), 1.0);
        let grow = tree.node(bar).unwrap().children[1];
        let rect = tree.node(grow).unwrap().computed.rect;
        assert!(rect.w > 100.0, "grow child should expand, got {}", rect.w);
    }

    #[test]
    fn root_fills_screen() {
        let mut tree = WidgetTree::new();
        run_layout(&mut tree, Vec2::new(640.0, 360.0), 1.0);
        let rect = tree.node(tree.root()).unwrap().computed.rect;
        assert_eq!(rect.w, 640.0);
        assert_eq!(rect.h, 360.0);
    }
}
