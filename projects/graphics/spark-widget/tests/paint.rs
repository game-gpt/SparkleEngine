//! 自 `src/paint/mod.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_types::{Color, Vec2};
use spark_widget::{
    layout::{LayoutSpec, Size, UiMetrics, run_layout},
    text::EstimateMeasurer,
    widgets::{button_widget, checkbox_widget, column, label_widget, slider_widget},
};

#[test]
fn paint_emits_commands_for_label_and_button() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    column()
        .child(label_widget().text("Hello"))
        .child(button_widget().text("OK").layout(LayoutSpec::default().with_height(Size::Px(36.0))))
        .child(checkbox_widget().text("On").checked(true))
        .child(slider_widget().value(0.5).layout(LayoutSpec::default().with_height(Size::Px(24.0)).with_width(Size::Px(120.0))))
        .mount(&mut tree, root)
        .unwrap();
    run_layout(&mut tree, Vec2::new(320.0, 240.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

    let theme = Theme::default();
    let motion = spark_widget::motion::MotionManager::new();
    let mut batch = UiRenderBatch::new();
    paint_tree_into(&tree, &theme, &motion, &mut spark_widget::asset::NullTextureResolver, &mut batch);
    let total = batch.command_count();
    assert!(
        total >= 4,
        "expected several UI commands, got quads={} tex={} texts={}",
        batch.quads.len(),
        batch.tex_quads.len(),
        batch.texts.len()
    );
}

#[test]
fn paint_image_emits_tex_quad() {
    use spark_asset::AssetId;
    use spark_renderer::TextureId;
    use spark_widget::{
        asset::{MapTextureResolver, UiImage},
        widgets::image_widget,
    };

    let mut tree = WidgetTree::new();
    let root = tree.root();
    let asset = AssetId(7);
    image_widget()
        .image(UiImage::new(asset).with_preferred_size(Vec2::new(48.0, 48.0)))
        .layout(LayoutSpec { width: Size::Px(48.0), height: Size::Px(48.0), ..LayoutSpec::default() })
        .mount(&mut tree, root)
        .unwrap();
    run_layout(&mut tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

    let mut textures = MapTextureResolver { asset, texture: TextureId(99), size: Vec2::new(48.0, 48.0) };
    let theme = Theme::default();
    let motion = spark_widget::motion::MotionManager::new();
    let mut batch = UiRenderBatch::new();
    paint_tree_into(&tree, &theme, &motion, &mut textures, &mut batch);
    assert!(!batch.tex_quads.is_empty(), "image should emit tex quads");
    assert_eq!(batch.tex_quads[0].texture, TextureId(99));
}

#[test]
fn paint_image_pending_draws_placeholder() {
    use spark_asset::AssetId;
    use spark_widget::{
        asset::{ResolvedTexture, UiImage, UiTextureResolver},
        widgets::image_widget,
    };

    struct PendingResolver {
        asset: AssetId,
    }
    impl UiTextureResolver for PendingResolver {
        fn resolve(&mut self, asset: AssetId) -> Option<ResolvedTexture> {
            if asset == self.asset { Some(ResolvedTexture::pending(Vec2::new(32.0, 32.0))) } else { None }
        }
    }

    let mut tree = WidgetTree::new();
    let root = tree.root();
    let asset = AssetId(3);
    image_widget()
        .image(UiImage::new(asset).with_preferred_size(Vec2::new(32.0, 32.0)))
        .layout(LayoutSpec { width: Size::Px(32.0), height: Size::Px(32.0), ..LayoutSpec::default() })
        .mount(&mut tree, root)
        .unwrap();
    run_layout(&mut tree, Vec2::new(100.0, 100.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

    let mut textures = PendingResolver { asset };
    let theme = Theme::default();
    let motion = spark_widget::motion::MotionManager::new();
    let mut batch = UiRenderBatch::new();
    paint_tree_into(&tree, &theme, &motion, &mut textures, &mut batch);
    assert!(batch.tex_quads.is_empty(), "pending must not sample texture");
    assert!(!batch.quads.is_empty(), "pending should draw placeholder fill");
}

#[test]
fn paint_tree_into_batch_then_flush() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    label_widget().text("batch").mount(&mut tree, root).unwrap();
    run_layout(&mut tree, Vec2::new(200.0, 80.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

    let theme = Theme::default();
    let motion = spark_widget::motion::MotionManager::new();
    let mut batch = UiRenderBatch::new();
    paint_tree_into(&tree, &theme, &motion, &mut spark_widget::asset::NullTextureResolver, &mut batch);
    assert!(batch.command_count() >= 1);
    assert!(!batch.texts.is_empty());

    // 兼容桥仍可用；正式路径由后端直读批次。
    let mut draw = spark_renderer::DrawList::new(Color::rgb(0.0, 0.0, 0.0));
    #[allow(deprecated)]
    batch.flush_hud(&mut draw);
    assert_eq!(batch.command_count(), 0);
    assert!(!draw.texts.is_empty());
}

#[test]
fn text_outlined_emits_outline_then_fill() {
    let mut batch = UiRenderBatch::new();
    batch.text_outlined(10.0, 20.0, 16.0, Color::rgb(1.0, 1.0, 1.0), Color::rgba(0.0, 0.0, 0.0, 0.8), &[(-1.0, 0.0), (1.0, 0.0)], "A");
    assert_eq!(batch.texts.len(), 3);
    assert!((batch.texts[0].pos.x - 9.0).abs() < f32::EPSILON);
    assert!((batch.texts[2].pos.x - 10.0).abs() < f32::EPSILON);
}

#[test]
fn editor_theme_is_compact_and_uses_quiet_buttons() {
    let theme = Theme::editor_dark();
    assert_eq!(theme.button_treatment, ButtonTreatment::Quiet);
    assert_eq!(theme.metrics.control_height, 24.0);
    assert!(theme.colors.control.r < theme.colors.control_hover.r);

    let mut node = WidgetNode::new(WidgetId(1), WidgetKind::Button);
    let idle = ComputedStyle::resolve_for(&theme, &node);
    assert_eq!(idle.background, theme.colors.control);
    assert_eq!(idle.foreground, theme.colors.foreground);

    node.state.selected = true;
    let selected = ComputedStyle::resolve_for(&theme, &node);
    assert_eq!(selected.background, theme.colors.selection);

    node.state.invalid = true;
    node.state.focused = true;
    let invalid = ComputedStyle::resolve_for(&theme, &node);
    assert_eq!(invalid.border, theme.colors.danger);
}

#[test]
fn declared_button_border_is_painted_without_focus() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let border = Color::rgb(0.8, 0.2, 0.1);
    button_widget()
        .text("border")
        .style(Style { border_color: Some(border), border_width: Some(2.0), ..Style::default() })
        .layout(LayoutSpec::default().with_width(Size::Px(100.0)).with_height(Size::Px(30.0)))
        .mount(&mut tree, root)
        .unwrap();
    run_layout(&mut tree, Vec2::new(120.0, 50.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

    let mut batch = UiRenderBatch::new();
    paint_tree_into(
        &tree,
        &Theme::editor_dark(),
        &spark_widget::motion::MotionManager::new(),
        &mut spark_widget::asset::NullTextureResolver,
        &mut batch,
    );

    let border_quads = batch.quads.iter().filter(|quad| quad.color == border).count();
    assert_eq!(border_quads, 4);
}
