//! 自 `src/binding.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_widget::{id::WidgetId, widgets::label_widget};

struct LabelVm {
    text: String,
    target: Option<WidgetId>,
}

impl UiViewModel for LabelVm {
    fn sync(&mut self, tree: &mut WidgetTree) {
        let Some(id) = self.target
        else {
            return;
        };
        set_text(tree, id, self.text.clone());
    }
}

#[test]
fn view_model_updates_label_text() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let id = label_widget().text("old").mount(&mut tree, root).unwrap();
    let mut vm = LabelVm { text: "new".into(), target: Some(id) };
    vm.sync(&mut tree);
    assert_eq!(tree.node(id).unwrap().content.text.as_deref(), Some("new"));
}

#[test]
fn bind_helpers_by_id_and_key() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    let id = label_widget().key("hud.hp").text("0").mount(&mut tree, root).unwrap();

    assert!(set_text_by_key(&mut tree, root, "hud.hp", "99"));
    assert_eq!(tree.node(id).unwrap().content.text.as_deref(), Some("99"));

    assert!(set_visible_by_key(&mut tree, root, "hud.hp", false));
    assert!(!tree.node(id).unwrap().state.visible);

    assert!(set_selected(&mut tree, id, true));
    assert!(tree.node(id).unwrap().state.selected);

    assert!(set_checked(&mut tree, id, true));
    assert!(tree.node(id).unwrap().content.checked);
    assert!(tree.node(id).unwrap().state.checked);

    assert!(set_value(&mut tree, id, 0.5));
    assert!((tree.node(id).unwrap().content.value - 0.5).abs() < f32::EPSILON);

    assert!(!set_text_by_key(&mut tree, root, "missing", "x"));
}

#[test]
fn disabled_and_menu_button() {
    use spark_types::Color;
    use spark_widget::menu_button;

    let mut tree = WidgetTree::new();
    let root = tree.root();
    let id = menu_button("Single Player", 24.0, Color::rgb(1.0, 1.0, 1.0))
        .key("title.single")
        .on_action("title.single")
        .disabled(true)
        .mount(&mut tree, root)
        .unwrap();

    let n = tree.node(id).unwrap();
    assert!(n.state.disabled);
    assert_eq!(n.content.text.as_deref(), Some("Single Player"));
    assert!(n.style.background.is_some_and(|c| c.a < 0.01));

    assert!(set_disabled(&mut tree, id, false));
    assert!(!tree.node(id).unwrap().state.disabled);
    assert!(set_disabled_by_key(&mut tree, root, "title.single", true));
    assert!(tree.node(id).unwrap().state.disabled);
}

#[test]
fn absolute_and_style_bind_helpers() {
    use spark_types::Color;
    use spark_widget::layout::Layout;

    let mut tree = WidgetTree::new();
    let root = tree.root();
    let id = label_widget().key("slot").text("x").mount(&mut tree, root).unwrap();

    assert!(set_absolute_offset(&mut tree, id, 12.0, 34.0));
    let n = tree.node(id).unwrap();
    assert_eq!(n.layout.kind, Layout::Absolute);
    assert_eq!(n.layout.offset_x, 12.0);
    assert_eq!(n.layout.offset_y, 34.0);

    assert!(set_absolute_bounds(&mut tree, id, 1.0, 2.0, 40.0, 50.0));
    let n = tree.node(id).unwrap();
    assert_eq!(n.layout.offset_x, 1.0);
    assert_eq!(n.layout.offset_y, 2.0);
    assert_eq!(n.layout.width, spark_widget::layout::Size::Px(40.0));
    assert_eq!(n.layout.height, spark_widget::layout::Size::Px(50.0));

    let gold = Color::rgb(1.0, 0.8, 0.2);
    assert!(set_foreground(&mut tree, id, Some(gold)));
    assert_eq!(tree.node(id).unwrap().style.foreground, Some(gold));
    assert!(set_background_by_key(&mut tree, root, "slot", Some(Color::rgba(0.0, 0.0, 0.0, 0.5))));
    assert!(tree.node(id).unwrap().style.background.is_some());

    assert!(set_opacity(&mut tree, id, Some(0.5)));
    assert_eq!(tree.node(id).unwrap().style.opacity, Some(0.5));
    assert!(set_font_size_by_key(&mut tree, root, "slot", Some(18.0)));
    assert_eq!(tree.node(id).unwrap().style.font_size, Some(18.0));
    assert!(set_corner_radius(&mut tree, id, Some(4.0)));
    assert_eq!(tree.node(id).unwrap().style.corner_radius, Some(4.0));
    assert!(set_size_px_by_key(&mut tree, root, "slot", 64.0, 32.0));
    assert_eq!(tree.node(id).unwrap().layout.width, spark_widget::layout::Size::Px(64.0));
}
