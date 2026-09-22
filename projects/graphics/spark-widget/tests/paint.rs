//! 自 `src/paint/mod.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_core::{Color, Vec2};
use spark_renderer::DrawList;
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
    let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
    paint_tree(&tree, &theme, &motion, &mut spark_widget::asset::NullTextureResolver, &mut draw);
    let total = draw.hud_quads.len() + draw.texts.len() + draw.quads.len();
    assert!(
        total >= 4,
        "expected several draw commands, got hud_quads={} texts={} quads={}",
        draw.hud_quads.len(),
        draw.texts.len(),
        draw.quads.len()
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
    let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
    paint_tree(&tree, &theme, &motion, &mut textures, &mut draw);
    assert!(!draw.hud_tex_quads.is_empty(), "image should emit hud tex quads");
    assert_eq!(draw.hud_tex_quads[0].texture, TextureId(99));
}
