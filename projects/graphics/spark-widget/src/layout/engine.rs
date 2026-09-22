//! measure / arrange 引擎。

use spark_core::{Rect, Vec2};

use crate::{
    id::WidgetId,
    node::{WidgetKind, WidgetNode},
    text::{TextMeasurer, TextStyle},
    tree::WidgetTree,
};

use super::{Align, Constraints, FlexDirection, Insets, Justify, Layout, LayoutSpec, Size, Size2, UiMetrics};

/// 对整棵树跑 measure + arrange。根节点填满安全区后的屏幕。
pub fn run_layout(tree: &mut WidgetTree, screen: Vec2, metrics: UiMetrics, measurer: &mut dyn TextMeasurer) {
    let root = tree.root();
    let safe = metrics.safe_area;
    let content_w = (screen.x - safe.horizontal()).max(0.0);
    let content_h = (screen.y - safe.vertical()).max(0.0);
    let screen_size = Size2::new(content_w, content_h);
    let constraints = Constraints::tight(screen_size);
    let _ = measure(tree, root, constraints, metrics, measurer);
    arrange(tree, root, Rect::new(safe.left, safe.top, content_w, content_h));
}

fn measure(tree: &mut WidgetTree, id: WidgetId, constraints: Constraints, metrics: UiMetrics, measurer: &mut dyn TextMeasurer) -> Size2 {
    let Some(node) = tree.node(id).cloned()
    else {
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
    let inner_constraints = apply_spec_limits(constraints.deflate(margin), &node.layout);
    let content_constraints = content_measure_constraints(inner_constraints.deflate(node.layout.padding), &node.layout);

    let content = measure_content(tree, &node, content_constraints, metrics, measurer);
    let padded = Size2::new(content.width + node.layout.padding.horizontal(), content.height + node.layout.padding.vertical());
    let sized = resolve_axis_sizes(padded, &node.layout, inner_constraints);
    let with_margin = Size2::new(sized.width + margin.horizontal(), sized.height + margin.vertical());
    let desired = if matches!(node.layout.width, Size::MaxContent) || matches!(node.layout.height, Size::MaxContent) {
        Size2::new(
            if matches!(node.layout.width, Size::MaxContent) {
                with_margin.width.max(constraints.min.width)
            }
            else {
                with_margin.width.clamp(constraints.min.width, constraints.max.width)
            },
            if matches!(node.layout.height, Size::MaxContent) {
                with_margin.height.max(constraints.min.height)
            }
            else {
                with_margin.height.clamp(constraints.min.height, constraints.max.height)
            },
        )
    }
    else {
        with_margin.clamp(constraints)
    };

    if let Some(n) = tree.node_mut(id) {
        n.computed.desired = desired;
    }
    desired
}

fn measure_content(
    tree: &mut WidgetTree,
    node: &WidgetNode,
    constraints: Constraints,
    metrics: UiMetrics,
    measurer: &mut dyn TextMeasurer,
) -> Size2 {
    if matches!(node.kind, WidgetKind::ScrollView | WidgetKind::ListView) {
        return measure_scroll(tree, node, constraints, metrics, measurer);
    }
    match node.layout.kind {
        Layout::Flex => measure_flex(tree, node, constraints, metrics, measurer),
        Layout::Grid => measure_grid(tree, node, constraints, metrics, measurer),
        Layout::Overlay | Layout::Stack | Layout::Anchor => measure_overlay(tree, node, constraints, metrics, measurer),
        Layout::Absolute => measure_absolute(tree, node, constraints, metrics, measurer),
    }
}

fn measure_scroll(
    tree: &mut WidgetTree,
    node: &WidgetNode,
    constraints: Constraints,
    metrics: UiMetrics,
    measurer: &mut dyn TextMeasurer,
) -> Size2 {
    let children = visible_children(tree, node);
    let child_constraints = Constraints::loose(Size2::new(f32::INFINITY, f32::INFINITY));
    let mut content = Size2::default();
    for child in &children {
        let size = measure(tree, *child, child_constraints, metrics, measurer);
        content.width = content.width.max(size.width);
        content.height = content.height.max(size.height);
    }
    if let Some(n) = tree.node_mut(node.id) {
        n.scroll.content_size = Vec2::new(content.width, content.height);
        // viewport / offset clamp 在 arrange 时用真实内容区更新，避免 measure 阶段用父级宽松约束把 offset 清零。
    }
    // 视口期望尺寸：优先固定/百分比，否则在约束内取内容大小。
    let width = match node.layout.width {
        Size::Px(v) => v,
        Size::Percent(p) if constraints.max.width.is_finite() => constraints.max.width * (p / 100.0),
        Size::Fill if constraints.max.width.is_finite() => constraints.max.width,
        _ => {
            if constraints.max.width.is_finite() {
                content.width.min(constraints.max.width)
            }
            else {
                content.width
            }
        }
    };
    let height = match node.layout.height {
        Size::Px(v) => v,
        Size::Percent(p) if constraints.max.height.is_finite() => constraints.max.height * (p / 100.0),
        Size::Fill if constraints.max.height.is_finite() => constraints.max.height,
        _ => {
            if constraints.max.height.is_finite() {
                content.height.min(constraints.max.height)
            }
            else {
                content.height
            }
        }
    };
    Size2::new(width, height).clamp(constraints)
}

fn measure_grid(
    tree: &mut WidgetTree,
    node: &WidgetNode,
    constraints: Constraints,
    metrics: UiMetrics,
    measurer: &mut dyn TextMeasurer,
) -> Size2 {
    let children = visible_children(tree, node);
    if children.is_empty() {
        return intrinsic_leaf(node, constraints, metrics, measurer);
    }
    let columns = node.layout.columns.max(1) as usize;
    let gap = node.layout.gap;
    let rows = children.len().div_ceil(columns);

    let cell_max_w = if constraints.max.width.is_finite() {
        let gaps = gap * (columns.saturating_sub(1) as f32);
        ((constraints.max.width - gaps) / columns as f32).max(0.0)
    }
    else {
        f32::INFINITY
    };
    let child_constraints = Constraints::loose(Size2::new(cell_max_w, f32::INFINITY));

    let mut col_widths = vec![0.0_f32; columns];
    let mut row_heights = vec![0.0_f32; rows];
    for (index, child) in children.iter().enumerate() {
        let size = measure(tree, *child, child_constraints, metrics, measurer);
        let col = index % columns;
        let row = index / columns;
        col_widths[col] = col_widths[col].max(size.width);
        row_heights[row] = row_heights[row].max(size.height);
    }

    let width = col_widths.iter().sum::<f32>() + gap * (columns.saturating_sub(1) as f32);
    let height = row_heights.iter().sum::<f32>() + gap * (rows.saturating_sub(1) as f32);
    Size2::new(width, height).clamp(constraints)
}

fn measure_flex(
    tree: &mut WidgetTree,
    node: &WidgetNode,
    constraints: Constraints,
    metrics: UiMetrics,
    measurer: &mut dyn TextMeasurer,
) -> Size2 {
    let children: Vec<WidgetId> = visible_children(tree, node);
    if children.is_empty() {
        return intrinsic_leaf(node, constraints, metrics, measurer);
    }

    let gap = node.layout.gap;
    let gap_total = if children.len() > 1 { gap * (children.len() as f32 - 1.0) } else { 0.0 };

    let vertical = node.layout.direction == FlexDirection::Column;
    let mut main = 0.0_f32;
    let mut cross = 0.0_f32;

    for child in &children {
        let child_constraints = if vertical {
            Constraints::loose(Size2::new(constraints.max.width, f32::INFINITY)).with_max_width(constraints.max.width)
        }
        else {
            Constraints::loose(Size2::new(f32::INFINITY, constraints.max.height)).with_max_height(constraints.max.height)
        };
        let size = measure(tree, *child, child_constraints, metrics, measurer);
        if vertical {
            main += size.height;
            cross = cross.max(size.width);
        }
        else {
            main += size.width;
            cross = cross.max(size.height);
        }
    }
    main += gap_total;

    if vertical {
        Size2::new(cross.min(constraints.max.width), main.min(constraints.max.height))
    }
    else {
        Size2::new(main.min(constraints.max.width), cross.min(constraints.max.height))
    }
}

fn measure_overlay(
    tree: &mut WidgetTree,
    node: &WidgetNode,
    constraints: Constraints,
    metrics: UiMetrics,
    measurer: &mut dyn TextMeasurer,
) -> Size2 {
    let children = visible_children(tree, node);
    if children.is_empty() {
        return intrinsic_leaf(node, constraints, metrics, measurer);
    }
    let mut size = Size2::default();
    for child in children {
        let child_size = measure(tree, child, constraints, metrics, measurer);
        size.width = size.width.max(child_size.width);
        size.height = size.height.max(child_size.height);
    }
    size.clamp(constraints)
}

fn measure_absolute(
    tree: &mut WidgetTree,
    node: &WidgetNode,
    constraints: Constraints,
    metrics: UiMetrics,
    measurer: &mut dyn TextMeasurer,
) -> Size2 {
    let children = visible_children(tree, node);
    let mut size = intrinsic_leaf(node, constraints, metrics, measurer);
    for child in children {
        let child_size = measure(tree, child, Constraints::loose(constraints.max), metrics, measurer);
        if let Some(child_node) = tree.node(child) {
            let right = child_node.layout.offset_x + child_size.width;
            let bottom = child_node.layout.offset_y + child_size.height;
            size.width = size.width.max(right);
            size.height = size.height.max(bottom);
        }
    }
    size.clamp(constraints)
}

fn intrinsic_leaf(node: &WidgetNode, constraints: Constraints, metrics: UiMetrics, measurer: &mut dyn TextMeasurer) -> Size2 {
    let text = node.content.text.as_deref().unwrap_or("");
    let scale = metrics.content_scale();
    let base = node.style.font_size.unwrap_or(match node.kind {
        WidgetKind::Label => 16.0,
        WidgetKind::Button => 16.0,
        _ => 14.0,
    });
    let style = TextStyle { size: base * scale, ..TextStyle::default() };
    let text_max_w = match node.layout.width {
        Size::MaxContent => None,
        Size::MinContent => Some((base * scale * 0.55).max(1.0)),
        _ if constraints.max.width.is_finite() => Some(constraints.max.width),
        _ => None,
    };
    let measured = if text.is_empty() {
        Size2::default()
    }
    else {
        let layout = measurer.measure(text, &style, text_max_w);
        Size2::new(layout.size.x, layout.size.y)
    };

    match node.kind {
        WidgetKind::Separator => {
            if node.layout.direction == FlexDirection::Row {
                Size2::new(constraints.max.width.min(1.0), 1.0)
            }
            else {
                Size2::new(1.0, constraints.max.height.min(1.0))
            }
        }
        WidgetKind::Spacer => Size2::default(),
        WidgetKind::Button => Size2::new((measured.width + 24.0 * scale).max(48.0 * scale), (measured.height + 12.0 * scale).max(32.0 * scale)),
        WidgetKind::Checkbox | WidgetKind::Toggle | WidgetKind::Radio => {
            Size2::new((measured.width + (18.0 + 8.0) * scale).max(24.0 * scale), measured.height.max(24.0 * scale))
        }
        WidgetKind::Slider => Size2::new(constraints.max.width.min(160.0 * scale).max(80.0 * scale), 24.0 * scale),
        WidgetKind::ProgressBar => Size2::new(constraints.max.width.min(160.0 * scale).max(80.0 * scale), 12.0 * scale),
        WidgetKind::TextField | WidgetKind::TextArea => Size2::new(constraints.max.width.min(200.0 * scale).max(80.0 * scale), 32.0 * scale),
        WidgetKind::Image => {
            let (w, h) = node
                .content
                .image
                .as_ref()
                .and_then(|img| img.preferred_size)
                .map(|s| (s.x * scale, s.y * scale))
                .unwrap_or((32.0 * scale, 32.0 * scale));
            Size2::new(w, h)
        }
        WidgetKind::Label => measured,
        _ => measured,
    }
}

fn resolve_axis_sizes(content: Size2, spec: &LayoutSpec, constraints: Constraints) -> Size2 {
    let width = resolve_size(spec.width, content.width, constraints.max.width, constraints.min.width);
    let height = resolve_size(spec.height, content.height, constraints.max.height, constraints.min.height);
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
    // MaxContent 允许超出父级 max（溢出）；其余轴仍受约束夹紧。
    if !matches!(spec.width, Size::MaxContent) {
        size.width = size.width.clamp(
            constraints.min.width,
            if constraints.max.width.is_finite() {
                constraints.max.width
            }
            else {
                size.width.max(constraints.min.width)
            },
        );
    }
    else {
        size.width = size.width.max(constraints.min.width);
    }
    if !matches!(spec.height, Size::MaxContent) {
        size.height = size.height.clamp(
            constraints.min.height,
            if constraints.max.height.is_finite() {
                constraints.max.height
            }
            else {
                size.height.max(constraints.min.height)
            },
        );
    }
    else {
        size.height = size.height.max(constraints.min.height);
    }
    size
}

fn resolve_size(spec: Size, content: f32, max: f32, min: f32) -> f32 {
    let value = match spec {
        // Fill 在 measure 阶段不抢无限空间，arrange 时再伸展。
        // MinContent / MaxContent 的内容已在 measure 阶段用对应约束算出。
        Size::Auto | Size::MinContent | Size::MaxContent | Size::Fill => content,
        Size::Px(v) => v,
        Size::Percent(p) => {
            if max.is_finite() {
                max * (p / 100.0)
            }
            else {
                content
            }
        }
    };
    match spec {
        Size::MaxContent | Size::MinContent => value.max(min),
        _ => {
            let upper = if max.is_finite() { max } else { value.max(min) };
            value.clamp(min, upper)
        }
    }
}

/// 按 `width` / `height` 的 Min/MaxContent 调整传给内容的测量约束。
fn content_measure_constraints(mut constraints: Constraints, spec: &LayoutSpec) -> Constraints {
    match spec.width {
        Size::MaxContent => constraints.max.width = f32::INFINITY,
        Size::MinContent => {
            constraints.max.width = constraints.max.width.min(8.0).max(constraints.min.width);
        }
        _ => {}
    }
    match spec.height {
        Size::MaxContent => constraints.max.height = f32::INFINITY,
        Size::MinContent => {
            constraints.max.height = constraints.max.height.min(8.0).max(constraints.min.height);
        }
        _ => {}
    }
    constraints
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
    let Some(node) = tree.node(id).cloned()
    else {
        return;
    };
    if !node.state.visible {
        return;
    }

    let margin = node.layout.margin;
    let border =
        Rect::new(rect.x + margin.left, rect.y + margin.top, (rect.w - margin.horizontal()).max(0.0), (rect.h - margin.vertical()).max(0.0));
    let content = deflate_rect(border, node.layout.padding);

    if let Some(n) = tree.node_mut(id) {
        n.computed.rect = border;
        n.computed.content_rect = content;
        n.computed.clip_rect = Some(border);
    }

    if matches!(node.kind, WidgetKind::ScrollView | WidgetKind::ListView) {
        arrange_scroll(tree, &node, content);
        return;
    }

    match node.layout.kind {
        Layout::Flex => arrange_flex(tree, &node, content),
        Layout::Grid => arrange_grid(tree, &node, content),
        Layout::Overlay | Layout::Stack | Layout::Anchor => arrange_overlay(tree, &node, content),
        Layout::Absolute => arrange_absolute(tree, &node, content),
    }
}

fn arrange_grid(tree: &mut WidgetTree, node: &WidgetNode, content: Rect) {
    let children = visible_children(tree, node);
    if children.is_empty() {
        return;
    }
    let columns = node.layout.columns.max(1) as usize;
    let gap = node.layout.gap;
    let rows = children.len().div_ceil(columns);

    let mut col_widths = vec![0.0_f32; columns];
    let mut row_heights = vec![0.0_f32; rows];
    for (index, child) in children.iter().enumerate() {
        let desired = tree.node(*child).map(|n| n.computed.desired).unwrap_or_default();
        let col = index % columns;
        let row = index / columns;
        col_widths[col] = col_widths[col].max(desired.width);
        row_heights[row] = row_heights[row].max(desired.height);
    }

    let used_w = col_widths.iter().sum::<f32>() + gap * (columns.saturating_sub(1) as f32);
    let free_w = (content.w - used_w).max(0.0);
    if free_w > 0.0 && columns > 0 {
        let add = free_w / columns as f32;
        for w in &mut col_widths {
            *w += add;
        }
    }

    let mut y = content.y;
    for row in 0..rows {
        let mut x = content.x;
        for col in 0..columns {
            let index = row * columns + col;
            if index >= children.len() {
                break;
            }
            let child = children[index];
            let cell_w = col_widths[col];
            let cell_h = row_heights[row];
            let desired = tree.node(child).map(|n| n.computed.desired).unwrap_or_default();
            let width = match tree.node(child).map(|n| n.layout.width) {
                Some(Size::Fill) => cell_w,
                _ => desired.width.min(cell_w),
            };
            let height = match tree.node(child).map(|n| n.layout.height) {
                Some(Size::Fill) => cell_h,
                _ => desired.height.min(cell_h),
            };
            let align = tree.node(child).map(|n| n.layout.align).unwrap_or(node.layout.align);
            let child_x = align_cross(x, cell_w, width, align);
            let child_y = align_cross(y, cell_h, height, align);
            arrange(tree, child, Rect::new(child_x, child_y, width, height));
            x += cell_w + gap;
        }
        y += row_heights[row] + gap;
    }
}

fn arrange_scroll(tree: &mut WidgetTree, node: &WidgetNode, content: Rect) {
    let offset = tree.node(node.id).map(|n| n.scroll.offset).unwrap_or(Vec2::ZERO);
    if let Some(n) = tree.node_mut(node.id) {
        n.scroll.viewport_size = Vec2::new(content.w, content.h);
        n.computed.clip_rect = Some(content);
        n.scroll.clamp_offset();
    }
    let offset = tree.node(node.id).map(|n| n.scroll.offset).unwrap_or(offset);
    for child in visible_children(tree, node) {
        let desired = tree.node(child).map(|n| n.computed.desired).unwrap_or_default();
        arrange(
            tree,
            child,
            Rect::new(content.x - offset.x, content.y - offset.y, desired.width.max(content.w), desired.height.max(content.h)),
        );
    }
}

fn arrange_flex(tree: &mut WidgetTree, node: &WidgetNode, content: Rect) {
    let children = visible_children(tree, node);
    if children.is_empty() {
        return;
    }

    let vertical = node.layout.direction == FlexDirection::Column;
    let gap = node.layout.gap;
    let gap_total = if children.len() > 1 { gap * (children.len() as f32 - 1.0) } else { 0.0 };

    let mut bases = Vec::with_capacity(children.len());
    let mut total_main = 0.0_f32;
    let mut total_grow = 0.0_f32;
    let mut total_shrink = 0.0_f32;

    for child in &children {
        let desired = tree.node(*child).map(|n| n.computed.desired).unwrap_or_default();
        let (grow, shrink, main_fill) = tree
            .node(*child)
            .map(|n| {
                let main_fill = if vertical { matches!(n.layout.height, Size::Fill) } else { matches!(n.layout.width, Size::Fill) };
                let grow = if main_fill { n.layout.flex_grow.max(1.0) } else { n.layout.flex_grow.max(0.0) };
                (grow, n.layout.flex_shrink.max(0.0), main_fill)
            })
            .unwrap_or((0.0, 0.0, false));
        let base_main = if main_fill {
            0.0
        }
        else if vertical {
            desired.height
        }
        else {
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
        }
        else if free < 0.0 && total_shrink > 0.0 && *shrink > 0.0 {
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
    }
    else {
        gap
    };

    let mut cursor = if vertical { content.y + justify_offset } else { content.x + justify_offset };

    for (index, child) in children.iter().enumerate() {
        let desired = bases[index].0;
        let main = mains[index];
        let child_layout = tree.node(*child).map(|n| n.layout.clone());
        let (child_w, child_h, x, y) = if vertical {
            let width = match (child_layout.as_ref().map(|l| l.width), node.layout.align) {
                (Some(Size::Fill), _) | (_, Align::Stretch) => content.w,
                _ => desired.width.min(content.w),
            };
            let x = align_cross(content.x, content.w, width, node.layout.align);
            (width, main, x, cursor)
        }
        else {
            let height = match (child_layout.as_ref().map(|l| l.height), node.layout.align) {
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
        let desired = tree.node(child).map(|n| n.computed.desired).unwrap_or_default();
        let align = tree.node(child).map(|n| n.layout.align).unwrap_or_default();
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
            let Some(child_node) = tree.node(child)
            else {
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
        arrange(tree, child, Rect::new(content.x + offset_x, content.y + offset_y, width, height));
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
    Rect::new(rect.x + padding.left, rect.y + padding.top, (rect.w - padding.horizontal()).max(0.0), (rect.h - padding.vertical()).max(0.0))
}

fn visible_children(tree: &WidgetTree, node: &WidgetNode) -> Vec<WidgetId> {
    node.children.iter().copied().filter(|id| tree.node(*id).map(|n| n.state.visible).unwrap_or(false)).collect()
}

#[cfg(test)]
mod tests {
    use super::{UiMetrics, *};
    use crate::{
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
        use crate::widgets::grid;

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
        use crate::widgets::scroll_view;

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
}
