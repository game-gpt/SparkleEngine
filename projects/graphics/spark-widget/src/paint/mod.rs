//! Paint traversal：retained 树 → [`UiRenderBatch`]。
//!
//! [`paint_tree`] 是兼容桥（内部 flush 到 DrawList HUD）；正式路径用 [`paint_tree_into`]。

mod batch;

pub use batch::UiRenderBatch;

use spark_renderer::DrawList;
use spark_types::{Color, Rect, Vec2};

use crate::{
    asset::UiTextureResolver,
    motion::{MotionManager, MotionSample},
    node::{WidgetKind, WidgetNode},
    style::{ComputedStyle, Theme},
    text::{self, TextStyle},
    tree::WidgetTree,
};

/// 兼容桥：遍历树写入批次后刷入 `draw` 的 HUD 层。
#[deprecated(note = "use paint_tree_into and submit UiRenderBatch via draw_ui")]
pub fn paint_tree(tree: &WidgetTree, theme: &Theme, motion: &MotionManager, textures: &mut dyn UiTextureResolver, draw: &mut DrawList) {
    let mut batch = UiRenderBatch::new();
    paint_tree_into(tree, theme, motion, textures, &mut batch);
    #[allow(deprecated)]
    batch.flush_hud(draw);
}

/// 遍历树，只写入 UI 批次（不碰世界 [`DrawList`]）。
pub fn paint_tree_into(
    tree: &WidgetTree,
    theme: &Theme,
    motion: &MotionManager,
    textures: &mut dyn UiTextureResolver,
    batch: &mut UiRenderBatch,
) {
    paint_node(tree, theme, motion, textures, batch, tree.root());
}

fn paint_node(
    tree: &WidgetTree,
    theme: &Theme,
    motion: &MotionManager,
    textures: &mut dyn UiTextureResolver,
    batch: &mut UiRenderBatch,
    id: crate::id::WidgetId,
) {
    let Some(node) = tree.node(id)
    else {
        return;
    };
    if !node.state.visible {
        return;
    }

    let mut style = ComputedStyle::resolve_for(theme, node);
    let sample = motion.sample(id);
    style.opacity *= sample.opacity;
    let is_scroll = matches!(node.kind, WidgetKind::ScrollView | WidgetKind::ListView);
    paint_widget(batch, theme, textures, node, &style, sample);

    if is_scroll {
        if let Some(clip) = node.computed.clip_rect.or(Some(node.computed.content_rect)) {
            batch.push_clip(clip);
        }
    }

    for child in &node.children {
        paint_node(tree, theme, motion, textures, batch, *child);
    }

    if is_scroll {
        batch.pop_clip();
        paint_scrollbar(batch, theme, node);
    }
}

fn paint_scrollbar(batch: &mut UiRenderBatch, theme: &Theme, node: &WidgetNode) {
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
    batch.fill_rect(Rect::new(track.x + track.w - bar_w, thumb_y, bar_w, thumb_h), with_alpha(theme.colors.border, 0.9));
}

fn paint_widget(
    batch: &mut UiRenderBatch,
    theme: &Theme,
    textures: &mut dyn UiTextureResolver,
    node: &WidgetNode,
    style: &ComputedStyle,
    sample: MotionSample,
) {
    let rect = scaled_rect(node.computed.rect, sample.scale);
    match node.kind {
        WidgetKind::Root | WidgetKind::Spacer => {}
        WidgetKind::Label => paint_label(batch, node, style, theme),
        WidgetKind::Button => paint_button(batch, textures, node, style, theme, rect),
        WidgetKind::Checkbox | WidgetKind::Toggle => paint_checkbox(batch, node, style, theme),
        WidgetKind::Radio => paint_radio(batch, node, style, theme),
        WidgetKind::Slider => paint_slider(batch, node, style, theme),
        WidgetKind::ProgressBar => paint_progress(batch, node, style, theme),
        WidgetKind::TextField | WidgetKind::TextArea => paint_text_field(batch, node, style, theme),
        WidgetKind::Separator => paint_separator(batch, node, style),
        WidgetKind::Image => paint_image(batch, textures, node, style, rect),
        WidgetKind::Container
        | WidgetKind::Panel
        | WidgetKind::ScrollView
        | WidgetKind::Modal
        | WidgetKind::Popup
        | WidgetKind::Toast
        | WidgetKind::Tooltip
        | WidgetKind::Custom
        | WidgetKind::ListView
        | WidgetKind::GridView
        | WidgetKind::TreeView
        | WidgetKind::TabView
        | WidgetKind::SplitView => {
            fill_if_opaque(batch, rect, style);
            if matches!(node.kind, WidgetKind::Panel | WidgetKind::Modal | WidgetKind::Popup | WidgetKind::Toast | WidgetKind::Tooltip)
                || node.state.focused
                || node.state.selected
                || node.state.invalid
            {
                paint_border(batch, rect, style, node.state.focused);
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

fn paint_image(batch: &mut UiRenderBatch, textures: &mut dyn UiTextureResolver, node: &WidgetNode, style: &ComputedStyle, rect: Rect) {
    fill_if_opaque(batch, rect, style);
    let Some(image) = node.content.image.as_ref()
    else {
        return;
    };
    let Some(resolved) = textures.resolve(image.asset)
    else {
        return;
    };
    if resolved.is_drawable() {
        let Some(texture) = resolved.texture
        else {
            return;
        };
        let tint = with_alpha(image.tint, style.opacity);
        let _sampler = resolved.sampler;
        batch.tex_rect(texture, rect, image.uv, tint);
        return;
    }
    if resolved.is_pending() {
        // 占位：半透明深灰，不阻塞布局；宿主完成上传后下一帧可 Resident。
        batch.fill_rect(rect, with_alpha(Color::rgb(0.18, 0.18, 0.20), style.opacity * 0.55));
    }
}

fn paint_label(batch: &mut UiRenderBatch, node: &WidgetNode, style: &ComputedStyle, _theme: &Theme) {
    let Some(text) = node.content.text.as_deref()
    else {
        return;
    };
    let size = style.font_size;
    let color = with_alpha(style.foreground, style.opacity);
    let rect = node.computed.content_rect;
    batch.text(rect.x, rect.y + 2.0, size, color, text);
}

fn paint_button(
    batch: &mut UiRenderBatch,
    textures: &mut dyn UiTextureResolver,
    node: &WidgetNode,
    style: &ComputedStyle,
    theme: &Theme,
    rect: Rect,
) {
    fill_if_opaque(batch, rect, style);
    paint_border(batch, rect, style, node.state.focused);
    // 图标按钮：有 `UiImage` 时居中贴图（选单 Play/Delete 等）。
    if let Some(image) = node.content.image.as_ref() {
        if let Some(resolved) = textures.resolve(image.asset) {
            if resolved.is_drawable() {
                if let Some(texture) = resolved.texture {
                    let pref = image.preferred_size.unwrap_or(resolved.size);
                    let iw = pref.x.min(rect.w).max(1.0);
                    let ih = pref.y.min(rect.h).max(1.0);
                    let ix = rect.x + ((rect.w - iw) * 0.5).max(0.0);
                    let iy = rect.y + ((rect.h - ih) * 0.5).max(0.0);
                    let tint = with_alpha(image.tint, style.opacity);
                    // 悬停略提亮。
                    let tint = if node.state.hovered {
                        Color::rgba((tint.r * 1.15).min(1.0), (tint.g * 1.15).min(1.0), (tint.b * 1.15).min(1.0), tint.a)
                    }
                    else {
                        tint
                    };
                    batch.tex_rect(texture, Rect::new(ix, iy, iw, ih), image.uv, tint);
                }
            }
        }
    }
    if let Some(text) = node.content.text.as_deref() {
        let size = style.font_size;
        let measured = text::measure_plain(text, &TextStyle { size, color: style.foreground, ..TextStyle::default() }, Some(rect.w));
        let x = rect.x + ((rect.w - measured.size.x) * 0.5).max(0.0);
        let y = rect.y + ((rect.h - measured.size.y) * 0.5).max(0.0);
        let fg = with_alpha(style.foreground, style.opacity);
        // 透明底菜单字：主题描边，贴近原版可读性。
        if style.background.a < 0.01 && !theme.menu_item.outline_offsets.is_empty() {
            let outline = with_alpha(theme.menu_item.outline, style.opacity);
            batch.text_outlined(x, y, size, fg, outline, &theme.menu_item.outline_offsets, text);
        }
        else {
            batch.text(x, y, size, fg, text);
        }
    }
}

fn paint_checkbox(batch: &mut UiRenderBatch, node: &WidgetNode, style: &ComputedStyle, theme: &Theme) {
    let box_size = 18.0;
    let rect = node.computed.content_rect;
    let box_rect = Rect::new(rect.x, rect.y + ((rect.h - box_size) * 0.5).max(0.0), box_size, box_size);
    batch.fill_rect(box_rect, with_alpha(theme.colors.background, style.opacity));
    stroke_rect(batch, box_rect, with_alpha(style.border, style.opacity), style.border_width);
    if node.content.checked || node.state.checked {
        let inset = 4.0;
        batch.fill_rect(
            Rect::new(box_rect.x + inset, box_rect.y + inset, box_size - inset * 2.0, box_size - inset * 2.0),
            with_alpha(style.accent, style.opacity),
        );
    }
    if let Some(text) = node.content.text.as_deref() {
        let size = theme.typography.body_size;
        batch.text(
            box_rect.x + box_size + 8.0,
            rect.y + ((rect.h - size) * 0.5).max(0.0),
            size,
            with_alpha(style.foreground, style.opacity),
            text,
        );
    }
    if node.state.focused {
        stroke_rect(batch, node.computed.rect, with_alpha(style.border, style.opacity), style.border_width.max(2.0));
    }
}

fn paint_radio(batch: &mut UiRenderBatch, node: &WidgetNode, style: &ComputedStyle, theme: &Theme) {
    // 首切：与 checkbox 相同视觉，后续再画圆。
    paint_checkbox(batch, node, style, theme);
}

fn paint_slider(batch: &mut UiRenderBatch, node: &WidgetNode, style: &ComputedStyle, theme: &Theme) {
    let rect = node.computed.content_rect;
    let track_h = 6.0;
    let track_y = rect.y + ((rect.h - track_h) * 0.5).max(0.0);
    let track = Rect::new(rect.x, track_y, rect.w, track_h);
    batch.fill_rect(track, with_alpha(theme.colors.track, style.opacity));

    let t = value_t(&node.content);
    let fill_w = (track.w * t).max(0.0);
    if fill_w > 0.0 {
        batch.fill_rect(Rect::new(track.x, track.y, fill_w, track.h), with_alpha(style.accent, style.opacity));
    }

    let thumb = 14.0;
    let thumb_x = track.x + fill_w - thumb * 0.5;
    let thumb_rect = Rect::new(thumb_x.clamp(track.x, track.x + track.w - thumb), track_y + track_h * 0.5 - thumb * 0.5, thumb, thumb);
    batch.fill_rect(thumb_rect, with_alpha(style.foreground, style.opacity));
    if node.state.focused {
        stroke_rect(batch, node.computed.rect, with_alpha(style.border, style.opacity), style.border_width.max(2.0));
    }
}

fn paint_progress(batch: &mut UiRenderBatch, node: &WidgetNode, style: &ComputedStyle, theme: &Theme) {
    let rect = node.computed.content_rect;
    batch.fill_rect(rect, with_alpha(theme.colors.track, style.opacity));
    let t = value_t(&node.content);
    let fill_w = (rect.w * t).max(0.0);
    if fill_w > 0.0 {
        batch.fill_rect(Rect::new(rect.x, rect.y, fill_w, rect.h), with_alpha(style.accent, style.opacity));
    }
}

fn paint_text_field(batch: &mut UiRenderBatch, node: &WidgetNode, style: &ComputedStyle, theme: &Theme) {
    fill_if_opaque(batch, node.computed.rect, style);
    stroke_rect(batch, node.computed.rect, with_alpha(style.border, style.opacity), style.border_width);
    let text = node.content.text.as_deref().unwrap_or("");
    let composition = node.content.composition.as_str();
    let size = theme.typography.body_size;
    let rect = node.computed.content_rect;
    let text_x = rect.x + 6.0;
    let text_y = rect.y + ((rect.h - size) * 0.5).max(0.0);
    let char_w = size * 0.55;
    let cursor = node.content.cursor.min(text.chars().count());

    if let Some(anchor) = node.content.sel_anchor {
        let a = anchor.min(text.chars().count());
        if a != cursor && composition.is_empty() {
            let (lo, hi) = if a < cursor { (a, cursor) } else { (cursor, a) };
            let x0 = text_x + lo as f32 * char_w;
            let x1 = text_x + hi as f32 * char_w;
            batch.fill_rect(Rect::new(x0, text_y, (x1 - x0).max(1.0), size), with_alpha(theme.colors.accent, style.opacity * 0.35));
        }
    }

    let before: String = text.chars().take(cursor).collect();
    let after: String = text.chars().skip(cursor).collect();
    batch.text(text_x, text_y, size, with_alpha(style.foreground, style.opacity), &before);
    let mut x = text_x + before.chars().count() as f32 * char_w;
    if !composition.is_empty() {
        let comp_w = composition.chars().count() as f32 * char_w;
        batch.text(x, text_y, size, with_alpha(theme.colors.accent, style.opacity), composition);
        batch.fill_rect(Rect::new(x, text_y + size - 2.0, comp_w.max(1.0), 2.0), with_alpha(theme.colors.accent, style.opacity));
        x += comp_w;
    }
    if !after.is_empty() {
        batch.text(x, text_y, size, with_alpha(style.foreground, style.opacity), &after);
    }

    if node.state.focused {
        stroke_rect(batch, node.computed.rect, with_alpha(style.border, style.opacity), style.border_width.max(2.0));
        let caret_x = if composition.is_empty() {
            text_x + cursor as f32 * char_w
        }
        else {
            text_x + (before.chars().count() + composition.chars().count()) as f32 * char_w
        };
        batch.fill_rect(Rect::new(caret_x, text_y, 1.5, size), with_alpha(theme.colors.accent, style.opacity));
    }
}

fn paint_separator(batch: &mut UiRenderBatch, node: &WidgetNode, style: &ComputedStyle) {
    batch.fill_rect(node.computed.rect, with_alpha(style.background, style.opacity));
}

fn fill_if_opaque(batch: &mut UiRenderBatch, rect: Rect, style: &ComputedStyle) {
    let color = with_alpha(style.background, style.opacity);
    if color.a > 0.001 {
        batch.fill_rect(rect, color);
    }
}

fn paint_border(batch: &mut UiRenderBatch, rect: Rect, style: &ComputedStyle, focused: bool) {
    if style.border.a <= 0.001 || style.border_width <= 0.0 {
        return;
    }
    let width = if focused { style.border_width.max(2.0) } else { style.border_width };
    stroke_rect(batch, rect, with_alpha(style.border, style.opacity), width);
}

fn stroke_rect(batch: &mut UiRenderBatch, rect: Rect, color: Color, thickness: f32) {
    if thickness <= 0.0 || color.a <= 0.001 {
        return;
    }
    let t = thickness.max(1.0);
    batch.fill_rect(Rect::new(rect.x, rect.y, rect.w, t), color);
    batch.fill_rect(Rect::new(rect.x, rect.y + rect.h - t, rect.w, t), color);
    batch.fill_rect(Rect::new(rect.x, rect.y, t, rect.h), color);
    batch.fill_rect(Rect::new(rect.x + rect.w - t, rect.y, t, rect.h), color);
}

fn with_alpha(mut color: Color, opacity: f32) -> Color {
    color.a *= opacity;
    color
}

fn value_t(content: &crate::node::WidgetContent) -> f32 {
    let span = (content.value_max - content.value_min).abs().max(f32::EPSILON);
    ((content.value - content.value_min) / span).clamp(0.0, 1.0)
}

/// 绘制上下文，供自定义 Widget 把命令写入同一 [`UiRenderBatch`]。
pub struct PaintContext<'a> {
    /// 本帧 UI 绘制批次。
    pub batch: &'a mut UiRenderBatch,
    /// 当前主题（色板与字号）。
    pub theme: &'a Theme,
}

impl<'a> PaintContext<'a> {
    /// 自定义绘制可用的光标位置占位（首切恒为原点）。
    pub fn cursor(&self) -> Vec2 {
        Vec2::ZERO
    }
}
