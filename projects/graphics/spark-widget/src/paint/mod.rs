//! Paint traversal：retained 树 → DrawList。

use spark_core::{Color, Rect, Vec2};
use spark_renderer::DrawList;

use crate::motion::{MotionManager, MotionSample};
use crate::node::{WidgetKind, WidgetNode};
use crate::style::{ComputedStyle, Theme};
use crate::text::{self, TextStyle};
use crate::tree::WidgetTree;

/// 遍历树并写入绘制命令。
pub fn paint_tree(
    tree: &WidgetTree,
    theme: &Theme,
    motion: &MotionManager,
    draw: &mut DrawList,
) {
    draw.begin_hud();
    paint_node(tree, theme, motion, draw, tree.root());
}

fn paint_node(
    tree: &WidgetTree,
    theme: &Theme,
    motion: &MotionManager,
    draw: &mut DrawList,
    id: crate::id::WidgetId,
) {
    let Some(node) = tree.node(id) else {
        return;
    };
    if !node.state.visible {
        return;
    }

    let mut style = ComputedStyle::resolve_for(theme, node);
    let sample = motion.sample(id);
    style.opacity *= sample.opacity;
    let is_scroll = matches!(node.kind, WidgetKind::ScrollView | WidgetKind::ListView);
    paint_widget(draw, theme, node, &style, sample);

    if is_scroll {
        if let Some(clip) = node.computed.clip_rect.or(Some(node.computed.content_rect)) {
            draw.push_clip(clip);
        }
    }

    for child in &node.children {
        paint_node(tree, theme, motion, draw, *child);
    }

    if is_scroll {
        draw.pop_clip();
        paint_scrollbar(draw, theme, node);
    }
}

fn paint_scrollbar(draw: &mut DrawList, theme: &Theme, node: &WidgetNode) {
    let scroll = &node.scroll;
    if scroll.content_size.y <= scroll.viewport_size.y + 0.5 {
        return;
    }
    let track = node.computed.content_rect;
    let bar_w = 6.0;
    let track_h = track.h.max(1.0);
    let ratio = (scroll.viewport_size.y / scroll.content_size.y).clamp(0.05, 1.0);
    let thumb_h = (track_h * ratio).max(12.0);
    let max_offset = (scroll.content_size.y - scroll.viewport_size.y).max(1.0);
    let t = (scroll.offset.y / max_offset).clamp(0.0, 1.0);
    let thumb_y = track.y + (track_h - thumb_h) * t;
    draw.fill_rect(
        Rect::new(track.x + track.w - bar_w, thumb_y, bar_w, thumb_h),
        with_alpha(theme.colors.border, 0.9),
    );
}

fn paint_widget(
    draw: &mut DrawList,
    theme: &Theme,
    node: &WidgetNode,
    style: &ComputedStyle,
    sample: MotionSample,
) {
    let rect = scaled_rect(node.computed.rect, sample.scale);
    match node.kind {
        WidgetKind::Root | WidgetKind::Spacer => {}
        WidgetKind::Label => paint_label(draw, node, style, theme),
        WidgetKind::Button => paint_button(draw, node, style, theme, rect),
        WidgetKind::Checkbox | WidgetKind::Toggle => paint_checkbox(draw, node, style, theme),
        WidgetKind::Radio => paint_radio(draw, node, style, theme),
        WidgetKind::Slider => paint_slider(draw, node, style, theme),
        WidgetKind::ProgressBar => paint_progress(draw, node, style, theme),
        WidgetKind::TextField | WidgetKind::TextArea => paint_text_field(draw, node, style, theme),
        WidgetKind::Separator => paint_separator(draw, node, style),
        WidgetKind::Container
        | WidgetKind::Panel
        | WidgetKind::ScrollView
        | WidgetKind::Modal
        | WidgetKind::Popup
        | WidgetKind::Toast
        | WidgetKind::Tooltip
        | WidgetKind::Custom
        | WidgetKind::Image
        | WidgetKind::ListView
        | WidgetKind::GridView
        | WidgetKind::TreeView
        | WidgetKind::TabView
        | WidgetKind::SplitView => {
            fill_if_opaque(draw, rect, style);
            if node.state.focused {
                stroke_rect(draw, rect, with_alpha(style.border, style.opacity), 2.0);
            }
        }
    }
}

fn scaled_rect(rect: Rect, scale: f32) -> Rect {
    if (scale - 1.0).abs() < 0.0001 {
        return rect;
    }
    let cx = rect.x + rect.w * 0.5;
    let cy = rect.y + rect.h * 0.5;
    let w = rect.w * scale;
    let h = rect.h * scale;
    Rect::new(cx - w * 0.5, cy - h * 0.5, w, h)
}

fn paint_label(draw: &mut DrawList, node: &WidgetNode, style: &ComputedStyle, theme: &Theme) {
    let Some(text) = node.content.text.as_deref() else {
        return;
    };
    let size = theme.typography.body_size;
    let color = with_alpha(style.foreground, style.opacity);
    let rect = node.computed.content_rect;
    draw.text(rect.x, rect.y + 2.0, size, color, text);
}

fn paint_button(
    draw: &mut DrawList,
    node: &WidgetNode,
    style: &ComputedStyle,
    theme: &Theme,
    rect: Rect,
) {
    fill_if_opaque(draw, rect, style);
    if node.state.focused {
        stroke_rect(draw, rect, with_alpha(style.border, style.opacity), 2.0);
    }
    if let Some(text) = node.content.text.as_deref() {
        let size = theme.typography.body_size;
        let measured = text::measure_plain(
            text,
            &TextStyle {
                size,
                color: style.foreground,
                ..TextStyle::default()
            },
            Some(rect.w),
        );
        let x = rect.x + ((rect.w - measured.size.x) * 0.5).max(0.0);
        let y = rect.y + ((rect.h - measured.size.y) * 0.5).max(0.0);
        draw.text(x, y, size, with_alpha(style.foreground, style.opacity), text);
    }
}

fn paint_checkbox(draw: &mut DrawList, node: &WidgetNode, style: &ComputedStyle, theme: &Theme) {
    let box_size = 18.0;
    let rect = node.computed.content_rect;
    let box_rect = Rect::new(rect.x, rect.y + ((rect.h - box_size) * 0.5).max(0.0), box_size, box_size);
    draw.fill_rect(box_rect, with_alpha(theme.colors.background, style.opacity));
    stroke_rect(draw, box_rect, with_alpha(style.border, style.opacity), 1.0);
    if node.content.checked || node.state.checked {
        let inset = 4.0;
        draw.fill_rect(
            Rect::new(
                box_rect.x + inset,
                box_rect.y + inset,
                box_size - inset * 2.0,
                box_size - inset * 2.0,
            ),
            with_alpha(style.accent, style.opacity),
        );
    }
    if let Some(text) = node.content.text.as_deref() {
        let size = theme.typography.body_size;
        draw.text(
            box_rect.x + box_size + 8.0,
            rect.y + ((rect.h - size) * 0.5).max(0.0),
            size,
            with_alpha(style.foreground, style.opacity),
            text,
        );
    }
    if node.state.focused {
        stroke_rect(draw, node.computed.rect, with_alpha(style.border, style.opacity), 2.0);
    }
}

fn paint_radio(draw: &mut DrawList, node: &WidgetNode, style: &ComputedStyle, theme: &Theme) {
    // 首切：与 checkbox 相同视觉，后续再画圆。
    paint_checkbox(draw, node, style, theme);
}

fn paint_slider(draw: &mut DrawList, node: &WidgetNode, style: &ComputedStyle, theme: &Theme) {
    let rect = node.computed.content_rect;
    let track_h = 6.0;
    let track_y = rect.y + ((rect.h - track_h) * 0.5).max(0.0);
    let track = Rect::new(rect.x, track_y, rect.w, track_h);
    draw.fill_rect(track, with_alpha(theme.colors.track, style.opacity));

    let t = value_t(&node.content);
    let fill_w = (track.w * t).max(0.0);
    if fill_w > 0.0 {
        draw.fill_rect(
            Rect::new(track.x, track.y, fill_w, track.h),
            with_alpha(style.accent, style.opacity),
        );
    }

    let thumb = 14.0;
    let thumb_x = track.x + fill_w - thumb * 0.5;
    let thumb_rect = Rect::new(
        thumb_x.clamp(track.x, track.x + track.w - thumb),
        track_y + track_h * 0.5 - thumb * 0.5,
        thumb,
        thumb,
    );
    draw.fill_rect(thumb_rect, with_alpha(style.foreground, style.opacity));
    if node.state.focused {
        stroke_rect(draw, node.computed.rect, with_alpha(style.border, style.opacity), 2.0);
    }
}

fn paint_progress(draw: &mut DrawList, node: &WidgetNode, style: &ComputedStyle, theme: &Theme) {
    let rect = node.computed.content_rect;
    draw.fill_rect(rect, with_alpha(theme.colors.track, style.opacity));
    let t = value_t(&node.content);
    let fill_w = (rect.w * t).max(0.0);
    if fill_w > 0.0 {
        draw.fill_rect(
            Rect::new(rect.x, rect.y, fill_w, rect.h),
            with_alpha(style.accent, style.opacity),
        );
    }
}

fn paint_text_field(draw: &mut DrawList, node: &WidgetNode, style: &ComputedStyle, theme: &Theme) {
    fill_if_opaque(draw, node.computed.rect, style);
    stroke_rect(draw, node.computed.rect, with_alpha(style.border, style.opacity), 1.0);
    let text = node.content.text.as_deref().unwrap_or("");
    let size = theme.typography.body_size;
    let rect = node.computed.content_rect;
    let text_x = rect.x + 6.0;
    let text_y = rect.y + ((rect.h - size) * 0.5).max(0.0);
    let char_w = size * 0.55;

    if let Some(anchor) = node.content.sel_anchor {
        let cursor = node.content.cursor.min(text.chars().count());
        let a = anchor.min(text.chars().count());
        if a != cursor {
            let (lo, hi) = if a < cursor { (a, cursor) } else { (cursor, a) };
            let x0 = text_x + lo as f32 * char_w;
            let x1 = text_x + hi as f32 * char_w;
            draw.fill_rect(
                Rect::new(x0, text_y, (x1 - x0).max(1.0), size),
                with_alpha(theme.colors.accent, style.opacity * 0.35),
            );
        }
    }

    draw.text(
        text_x,
        text_y,
        size,
        with_alpha(style.foreground, style.opacity),
        text,
    );

    if node.state.focused {
        stroke_rect(draw, node.computed.rect, with_alpha(theme.colors.accent, style.opacity), 2.0);
        let cursor = node.content.cursor.min(text.chars().count());
        let caret_x = text_x + cursor as f32 * char_w;
        draw.fill_rect(
            Rect::new(caret_x, text_y, 1.5, size),
            with_alpha(theme.colors.accent, style.opacity),
        );
    }
}

fn paint_separator(draw: &mut DrawList, node: &WidgetNode, style: &ComputedStyle) {
    draw.fill_rect(node.computed.rect, with_alpha(style.background, style.opacity));
}

fn fill_if_opaque(draw: &mut DrawList, rect: Rect, style: &ComputedStyle) {
    let color = with_alpha(style.background, style.opacity);
    if color.a > 0.001 {
        draw.fill_rect(rect, color);
    }
}

fn stroke_rect(draw: &mut DrawList, rect: Rect, color: Color, thickness: f32) {
    let t = thickness.max(1.0);
    draw.fill_rect(Rect::new(rect.x, rect.y, rect.w, t), color);
    draw.fill_rect(Rect::new(rect.x, rect.y + rect.h - t, rect.w, t), color);
    draw.fill_rect(Rect::new(rect.x, rect.y, t, rect.h), color);
    draw.fill_rect(Rect::new(rect.x + rect.w - t, rect.y, t, rect.h), color);
}

fn with_alpha(mut color: Color, opacity: f32) -> Color {
    color.a *= opacity;
    color
}

fn value_t(content: &crate::node::WidgetContent) -> f32 {
    let span = (content.value_max - content.value_min).abs().max(f32::EPSILON);
    ((content.value - content.value_min) / span).clamp(0.0, 1.0)
}

/// 绘制上下文，供自定义 Widget 使用。
pub struct PaintContext<'a> {
    pub draw: &'a mut DrawList,
    pub theme: &'a Theme,
}

impl<'a> PaintContext<'a> {
    pub fn cursor(&self) -> Vec2 {
        Vec2::ZERO
    }
}

#[cfg(test)]
mod tests {
    use crate::text::EstimateMeasurer;
    use super::*;
    use crate::layout::{run_layout, LayoutSpec, Size};
    use crate::widgets::{button_widget, checkbox_widget, column, label_widget, slider_widget};

    #[test]
    fn paint_emits_commands_for_label_and_button() {
        let mut tree = WidgetTree::new();
        let root = tree.root();
        column()
            .child(label_widget().text("Hello"))
            .child(
                button_widget()
                    .text("OK")
                    .layout(LayoutSpec::default().with_height(Size::Px(36.0))),
            )
            .child(checkbox_widget().text("On").checked(true))
            .child(
                slider_widget()
                    .value(0.5)
                    .layout(LayoutSpec::default().with_height(Size::Px(24.0)).with_width(Size::Px(120.0))),
            )
            .mount(&mut tree, root)
            .unwrap();
        run_layout(&mut tree, Vec2::new(320.0, 240.0), 1.0, &mut EstimateMeasurer);

        let theme = Theme::default();
        let motion = crate::motion::MotionManager::new();
        let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
        paint_tree(&tree, &theme, &motion, &mut draw);
        let total = draw.hud_quads.len() + draw.texts.len() + draw.quads.len();
        assert!(
            total >= 4,
            "expected several draw commands, got hud_quads={} texts={} quads={}",
            draw.hud_quads.len(),
            draw.texts.len(),
            draw.quads.len()
        );
    }
}
