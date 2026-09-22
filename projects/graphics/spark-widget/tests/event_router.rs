//! 自 `src/event/router.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_input::{ButtonState, Input, Key, MouseBtn};
use spark_types::Vec2;
use spark_widget::{
    layout::{LayoutSpec, Size, UiMetrics, run_layout},
    text::EstimateMeasurer,
    widgets::{button_widget, checkbox_widget, column},
};

fn frame<'a>(input: &'a Input, w: f32, h: f32) -> UiFrame<'a> {
    UiFrame {
        dt: 1.0 / 60.0,
        screen_size: Vec2::new(w, h),
        dpi_scale: 1.0,
        ui_scale: 1.0,
        safe_area: spark_widget::layout::Insets::default(),
        input,
    }
}

#[test]
fn hit_test_prefers_topmost_child() {
    let mut tree = WidgetTree::new();
    let root = tree.root();
    // 根下绝对叠放两个按钮，后挂载者命中优先。
    let panel = tree.mount(root, spark_widget::node::WidgetKind::Container).unwrap();
    if let Some(node) = tree.node_mut(panel) {
        node.layout =
            LayoutSpec { kind: spark_widget::layout::Layout::Absolute, width: Size::Fill, height: Size::Fill, ..LayoutSpec::default() };
    }
    let a = button_widget()
        .key("a")
        .text("A")
        .layout(LayoutSpec { width: Size::Px(100.0), height: Size::Px(40.0), ..LayoutSpec::default() })
        .mount(&mut tree, panel)
        .unwrap();
    let b = button_widget()
        .key("b")
        .text("B")
        .layout(LayoutSpec { width: Size::Px(100.0), height: Size::Px(40.0), ..LayoutSpec::default() })
        .mount(&mut tree, panel)
        .unwrap();
    run_layout(&mut tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let point = tree.node(b).unwrap().computed.rect.center();
    let hit = hit_test(&tree, root, point).unwrap();
    assert_eq!(hit, b);
    assert_ne!(hit, a);
}

#[test]
fn click_emits_command_and_blocks_input() {
    let mut runtime = UiRuntime::new();
    let root = runtime.tree.root();
    button_widget()
        .text("Go")
        .on_click(UiCommand::Custom(7))
        .layout(LayoutSpec { width: Size::Px(80.0), height: Size::Px(40.0), ..LayoutSpec::default() })
        .mount(&mut runtime.tree, root)
        .unwrap();
    run_layout(&mut runtime.tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

    let btn = runtime.tree.node(root).unwrap().children[0];
    let center = runtime.tree.node(btn).unwrap().computed.rect.center();

    let mut input = Input::default();
    input.on_cursor(center.x, center.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
    let f = frame(&input, 200.0, 200.0);
    runtime.begin_frame(&f);
    runtime.dispatch_input(&f);

    input.begin_frame();
    input.on_cursor(center.x, center.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
    let f = frame(&input, 200.0, 200.0);
    runtime.dispatch_input(&f);

    let commands: Vec<_> = runtime.drain_commands().collect();
    assert!(matches!(commands.as_slice(), [UiCommand::Custom(7)]));
    assert!(runtime.state.input_blocked);
}

#[test]
fn checkbox_click_toggles_checked() {
    let mut runtime = UiRuntime::new();
    let root = runtime.tree.root();
    checkbox_widget()
        .text("X")
        .layout(LayoutSpec { width: Size::Px(120.0), height: Size::Px(28.0), ..LayoutSpec::default() })
        .mount(&mut runtime.tree, root)
        .unwrap();
    run_layout(&mut runtime.tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let id = runtime.tree.node(root).unwrap().children[0];
    let center = runtime.tree.node(id).unwrap().computed.rect.center();

    let mut input = Input::default();
    input.on_cursor(center.x, center.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
    let f = frame(&input, 200.0, 200.0);
    runtime.begin_frame(&f);
    runtime.dispatch_input(&f);
    input.begin_frame();
    input.on_cursor(center.x, center.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
    let f = frame(&input, 200.0, 200.0);
    runtime.dispatch_input(&f);

    assert!(runtime.tree.node(id).unwrap().content.checked);
}

#[test]
fn modal_blocks_hits_outside_and_escape_closes() {
    use spark_widget::{overlay::OverlayLayer, widgets::modal_widget};

    let mut runtime = UiRuntime::new();
    let root = runtime.tree.root();
    button_widget()
        .text("Behind")
        .on_click(UiCommand::Custom(1))
        .layout(LayoutSpec { width: Size::Px(80.0), height: Size::Px(40.0), ..LayoutSpec::default() })
        .mount(&mut runtime.tree, root)
        .unwrap();
    runtime
        .open_overlay(
            OverlayLayer::Modal,
            modal_widget().child(button_widget().text("Modal").on_click(UiCommand::Custom(2)).layout(LayoutSpec {
                width: Size::Px(100.0),
                height: Size::Px(40.0),
                ..LayoutSpec::default()
            })),
        )
        .unwrap();
    run_layout(&mut runtime.tree, Vec2::new(400.0, 300.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

    let behind = runtime.tree.node(root).unwrap().children[0];
    let behind_center = runtime.tree.node(behind).unwrap().computed.rect.center();

    let mut input = Input::default();
    input.on_cursor(behind_center.x, behind_center.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
    let f = frame(&input, 400.0, 300.0);
    runtime.begin_frame(&f);
    runtime.dispatch_input(&f);
    input.begin_frame();
    input.on_cursor(behind_center.x, behind_center.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
    let f = frame(&input, 400.0, 300.0);
    runtime.dispatch_input(&f);
    assert!(runtime.drain_commands().next().is_none(), "clicks under modal must not reach behind button");
    assert!(runtime.state.input_blocked);

    input.begin_frame();
    input.on_key(Key::Escape, ButtonState::Pressed);
    let f = frame(&input, 400.0, 300.0);
    runtime.dispatch_input(&f);
    assert!(runtime.overlays.is_empty());
}

#[test]
fn toast_expires_after_ttl() {
    use spark_widget::widgets::toast_widget;

    let mut runtime = UiRuntime::new();
    runtime.show_toast(toast_widget().text("Saved"), 0.5).unwrap();
    assert!(!runtime.overlays.is_empty());
    runtime.update(0.6);
    assert!(runtime.overlays.is_empty());
}

#[test]
fn drag_drop_emits_command() {
    use spark_widget::widgets::panel;

    let mut runtime = UiRuntime::new();
    let root = runtime.tree.root();
    let src = panel()
        .drag_source(true)
        .layout(LayoutSpec { width: Size::Px(40.0), height: Size::Px(40.0), ..LayoutSpec::default() })
        .mount(&mut runtime.tree, root)
        .unwrap();
    let dst = panel()
        .drop_target(true)
        .layout(LayoutSpec {
            width: Size::Px(40.0),
            height: Size::Px(40.0),
            offset_x: 80.0,
            ..LayoutSpec { kind: spark_widget::layout::Layout::Absolute, ..LayoutSpec::default() }
        })
        .mount(&mut runtime.tree, root)
        .unwrap();
    // Put both under absolute parent for predictable positions.
    if let Some(node) = runtime.tree.node_mut(root) {
        node.layout.kind = spark_widget::layout::Layout::Absolute;
    }
    if let Some(node) = runtime.tree.node_mut(src) {
        node.layout.kind = spark_widget::layout::Layout::Absolute;
        node.layout.offset_x = 0.0;
        node.layout.offset_y = 0.0;
    }
    if let Some(node) = runtime.tree.node_mut(dst) {
        node.layout.offset_x = 80.0;
        node.layout.offset_y = 0.0;
    }
    run_layout(&mut runtime.tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

    let src_c = runtime.tree.node(src).unwrap().computed.rect.center();
    let dst_c = runtime.tree.node(dst).unwrap().computed.rect.center();

    let mut input = Input::default();
    input.on_cursor(src_c.x, src_c.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
    let f = frame(&input, 200.0, 200.0);
    runtime.begin_frame(&f);
    runtime.dispatch_input(&f);

    input.begin_frame();
    input.on_cursor(src_c.x + 20.0, src_c.y);
    // keep button down via mouse_down set
    // Input doesn't expose setting mouse_down directly - use press without begin clearing down
    // After begin_frame, mouse_down is preserved from previous press until release.
    let f = frame(&input, 200.0, 200.0);
    runtime.dispatch_input(&f);

    input.begin_frame();
    input.on_cursor(dst_c.x, dst_c.y);
    let f = frame(&input, 200.0, 200.0);
    runtime.dispatch_input(&f);

    input.begin_frame();
    input.on_cursor(dst_c.x, dst_c.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
    let f = frame(&input, 200.0, 200.0);
    runtime.dispatch_input(&f);

    let commands: Vec<_> = runtime.drain_commands().collect();
    assert!(
        matches!(
            commands.as_slice(),
            [UiCommand::Drop {
                source,
                target
            }] if *source == src && *target == dst
        ),
        "expected Drop command, got {commands:?}"
    );
}

#[test]
fn popup_closes_on_outside_click() {
    use spark_widget::widgets::{button_widget, label_widget, popup_widget};

    let mut runtime = UiRuntime::new();
    let root = runtime.tree.root();
    let anchor = button_widget()
        .text("Menu")
        .layout(LayoutSpec { width: Size::Px(80.0), height: Size::Px(32.0), ..LayoutSpec::default() })
        .mount(&mut runtime.tree, root)
        .unwrap();
    runtime.show_popup(anchor, popup_widget().child(label_widget().text("Item"))).unwrap();
    assert!(!runtime.overlays.is_empty());

    run_layout(&mut runtime.tree, Vec2::new(400.0, 300.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

    let mut input = Input::default();
    input.on_cursor(390.0, 290.0);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
    let f = frame(&input, 400.0, 300.0);
    runtime.begin_frame(&f);
    runtime.dispatch_input(&f);
    assert!(runtime.overlays.is_empty(), "outside click should dismiss popup");
}

#[test]
fn hud_chrome_does_not_block_world_input() {
    use spark_widget::{
        runtime::UiLayer,
        widgets::{button_widget, column, label_widget},
    };

    let mut runtime = UiRuntime::new();
    runtime
        .mount_hud(
            column()
                .layer(UiLayer::Hud)
                .layout(LayoutSpec { width: Size::Px(120.0), height: Size::Px(100.0), ..LayoutSpec::vertical().with_gap(8.0) })
                .child(label_widget().text("HP").layout(LayoutSpec::default().with_height(Size::Px(24.0))))
                .child(button_widget().text("Bag").on_click(UiCommand::Custom(9)).layout(LayoutSpec::default().with_height(Size::Px(32.0)))),
        )
        .unwrap();
    run_layout(&mut runtime.tree, Vec2::new(400.0, 300.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

    let hud = runtime.hud_root().unwrap();
    assert_eq!(runtime.tree.node(hud).unwrap().layer, UiLayer::Hud);
    let label = runtime.tree.node(hud).unwrap().children[0];
    assert_eq!(runtime.tree.node(label).unwrap().layer, UiLayer::Hud);
    let label_center = runtime.tree.node(label).unwrap().computed.rect.center();

    let mut input = Input::default();
    input.on_cursor(label_center.x, label_center.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
    let f = frame(&input, 400.0, 300.0);
    runtime.begin_frame(&f);
    runtime.dispatch_input(&f);
    assert!(!runtime.state.input_blocked, "HUD label must not block world input");

    let bag = runtime.tree.node(hud).unwrap().children[1];
    let bag_center = runtime.tree.node(bag).unwrap().computed.rect.center();
    input.begin_frame();
    input.on_cursor(bag_center.x, bag_center.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
    let f = frame(&input, 400.0, 300.0);
    runtime.begin_frame(&f);
    runtime.dispatch_input(&f);
    assert!(runtime.state.input_blocked, "HUD button should block world input");
}

#[test]
fn radio_group_is_exclusive() {
    use spark_widget::widgets::radio_widget;

    let mut runtime = UiRuntime::new();
    let root = runtime.tree.root();
    column().child(radio_widget().text("A").checked(true)).child(radio_widget().text("B")).mount(&mut runtime.tree, root).unwrap();
    run_layout(&mut runtime.tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let group = runtime.tree.node(root).unwrap().children[0];
    let a = runtime.tree.node(group).unwrap().children[0];
    let b = runtime.tree.node(group).unwrap().children[1];
    let center = runtime.tree.node(b).unwrap().computed.rect.center();

    let mut input = Input::default();
    input.on_cursor(center.x, center.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
    let f = frame(&input, 200.0, 200.0);
    runtime.begin_frame(&f);
    runtime.dispatch_input(&f);
    input.begin_frame();
    input.on_cursor(center.x, center.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
    let f = frame(&input, 200.0, 200.0);
    runtime.dispatch_input(&f);

    assert!(!runtime.tree.node(a).unwrap().content.checked);
    assert!(runtime.tree.node(b).unwrap().content.checked);
}

#[test]
fn text_field_accepts_typed_chars() {
    use spark_widget::widgets::text_field_widget;

    let mut runtime = UiRuntime::new();
    let root = runtime.tree.root();
    let id = text_field_widget()
        .text("")
        .layout(LayoutSpec { width: Size::Px(120.0), height: Size::Px(32.0), ..LayoutSpec::default() })
        .mount(&mut runtime.tree, root)
        .unwrap();
    run_layout(&mut runtime.tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    focus::set_focus(&mut runtime.tree, &mut runtime.focus, Some(id));

    let mut input = Input::default();
    input.on_text("hi");
    let f = frame(&input, 200.0, 200.0);
    runtime.begin_frame(&f);
    runtime.dispatch_input(&f);
    assert_eq!(runtime.tree.node(id).unwrap().content.text.as_deref(), Some("hi"));
    assert_eq!(runtime.tree.node(id).unwrap().content.cursor, 2);

    input.begin_frame();
    input.on_key(Key::Left, ButtonState::Pressed);
    let f = frame(&input, 200.0, 200.0);
    runtime.dispatch_input(&f);
    assert_eq!(runtime.tree.node(id).unwrap().content.cursor, 1);

    input.begin_frame();
    input.on_key(Key::Left, ButtonState::Released);
    input.begin_frame();
    input.on_key(Key::Backspace, ButtonState::Pressed);
    let f = frame(&input, 200.0, 200.0);
    runtime.dispatch_input(&f);
    assert_eq!(runtime.tree.node(id).unwrap().content.text.as_deref(), Some("i"));
}

#[test]
fn modal_tab_traps_focus_inside() {
    let mut runtime = UiRuntime::new();
    let root = runtime.tree.root();
    let outside = button_widget()
        .text("out")
        .layout(LayoutSpec { width: Size::Px(80.0), height: Size::Px(30.0), ..LayoutSpec::default() })
        .mount(&mut runtime.tree, root)
        .unwrap();
    runtime
        .open_overlay(
            OverlayLayer::Modal,
            modal_widget().child(
                column()
                    .layout(LayoutSpec { width: Size::Px(200.0), height: Size::Px(120.0), ..LayoutSpec::default() })
                    .child(button_widget().text("a").layout(LayoutSpec {
                        width: Size::Px(80.0),
                        height: Size::Px(30.0),
                        ..LayoutSpec::default()
                    }))
                    .child(button_widget().text("b").layout(LayoutSpec {
                        width: Size::Px(80.0),
                        height: Size::Px(30.0),
                        ..LayoutSpec::default()
                    })),
            ),
        )
        .unwrap();
    run_layout(&mut runtime.tree, Vec2::new(400.0, 300.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

    let modal = runtime.overlays.top_modal().unwrap();
    let first = focus::collect_focusable_in(&runtime.tree, modal)[0];
    assert_eq!(runtime.focus.focused, Some(first));
    assert_ne!(runtime.focus.focused, Some(outside));

    let mut input = Input::default();
    input.on_key(Key::Tab, ButtonState::Pressed);
    let f = frame(&input, 400.0, 300.0);
    runtime.dispatch_input(&f);
    let second = runtime.focus.focused.unwrap();
    assert_ne!(second, first);
    assert_ne!(second, outside);

    input.begin_frame();
    input.on_key(Key::Tab, ButtonState::Released);
    input.begin_frame();
    input.on_key(Key::Tab, ButtonState::Pressed);
    let f = frame(&input, 400.0, 300.0);
    runtime.dispatch_input(&f);
    assert_eq!(runtime.focus.focused, Some(first));
}

#[test]
fn text_field_syncs_ime_composition() {
    let mut runtime = UiRuntime::new();
    let root = runtime.tree.root();
    let id = text_field_widget()
        .text("ab")
        .layout(LayoutSpec { width: Size::Px(120.0), height: Size::Px(32.0), ..LayoutSpec::default() })
        .mount(&mut runtime.tree, root)
        .unwrap();
    run_layout(&mut runtime.tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    focus::set_focus(&mut runtime.tree, &mut runtime.focus, Some(id));
    if let Some(n) = runtime.tree.node_mut(id) {
        n.content.cursor = 2;
    }

    let mut input = Input::default();
    input.on_ime_preedit("你", Some((0, 3)));
    let f = frame(&input, 200.0, 200.0);
    runtime.begin_frame(&f);
    runtime.dispatch_input(&f);
    assert_eq!(runtime.tree.node(id).unwrap().content.composition.as_str(), "你");
    assert!(runtime.state.input_blocked);

    input.begin_frame();
    input.on_ime_commit("你好");
    let f = frame(&input, 200.0, 200.0);
    runtime.dispatch_input(&f);
    assert!(runtime.tree.node(id).unwrap().content.composition.is_empty());
    assert_eq!(runtime.tree.node(id).unwrap().content.text.as_deref(), Some("ab你好"));
}

#[test]
fn tab_moves_focus_between_buttons() {
    let mut runtime = UiRuntime::new();
    let root = runtime.tree.root();
    column()
        .child(button_widget().text("1").layout(LayoutSpec::default().with_height(Size::Px(30.0))))
        .child(button_widget().text("2").layout(LayoutSpec::default().with_height(Size::Px(30.0))))
        .mount(&mut runtime.tree, root)
        .unwrap();
    run_layout(&mut runtime.tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

    let mut input = Input::default();
    input.on_key(Key::Tab, ButtonState::Pressed);
    let f = frame(&input, 200.0, 200.0);
    runtime.dispatch_input(&f);
    let first = runtime.focus.focused.expect("tab focuses first button");

    focus::focus_next(&runtime.tree, &mut runtime.focus);
    let second = runtime.focus.focused.expect("focus moves to next");
    assert_ne!(first, second);

    // 再按 Tab（先松开）应继续循环。
    input.begin_frame();
    input.on_key(Key::Tab, ButtonState::Released);
    input.begin_frame();
    input.on_key(Key::Tab, ButtonState::Pressed);
    let f = frame(&input, 200.0, 200.0);
    runtime.dispatch_input(&f);
    assert_eq!(runtime.focus.focused, Some(first));
}

#[test]
fn prevent_click_default_skips_checkbox_toggle() {
    let mut runtime = UiRuntime::new();
    let root = runtime.tree.root();
    let id = checkbox_widget()
        .text("X")
        .prevent_click_default(true)
        .layout(LayoutSpec { width: Size::Px(120.0), height: Size::Px(28.0), ..LayoutSpec::default() })
        .mount(&mut runtime.tree, root)
        .unwrap();
    run_layout(&mut runtime.tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let center = runtime.tree.node(id).unwrap().computed.rect.center();

    let mut input = Input::default();
    input.on_cursor(center.x, center.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
    let f = frame(&input, 200.0, 200.0);
    runtime.begin_frame(&f);
    runtime.dispatch_input(&f);
    input.begin_frame();
    input.on_cursor(center.x, center.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
    let f = frame(&input, 200.0, 200.0);
    runtime.dispatch_input(&f);

    assert!(!runtime.tree.node(id).unwrap().content.checked);
    assert!(runtime.inspector.traces().iter().any(|e| e.kind == "target"));
}

#[test]
fn stop_click_propagation_halts_bubble_trace() {
    let mut runtime = UiRuntime::new();
    let root = runtime.tree.root();
    let col = column()
        .child(button_widget().text("Go").stop_click_propagation(true).on_click(UiCommand::Custom(3)).layout(LayoutSpec {
            width: Size::Px(80.0),
            height: Size::Px(40.0),
            ..LayoutSpec::default()
        }))
        .mount(&mut runtime.tree, root)
        .unwrap();
    run_layout(&mut runtime.tree, Vec2::new(200.0, 200.0), UiMetrics::new(1.0), &mut EstimateMeasurer);
    let btn = runtime.tree.node(col).unwrap().children[0];
    let center = runtime.tree.node(btn).unwrap().computed.rect.center();

    let mut input = Input::default();
    input.on_cursor(center.x, center.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
    let f = frame(&input, 200.0, 200.0);
    runtime.begin_frame(&f);
    runtime.dispatch_input(&f);
    input.begin_frame();
    input.on_cursor(center.x, center.y);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Released);
    let f = frame(&input, 200.0, 200.0);
    runtime.dispatch_input(&f);

    assert!(matches!(runtime.drain_commands().next(), Some(UiCommand::Custom(3))));
    assert!(!runtime.inspector.traces().iter().any(|e| e.kind == "bubble"));
    assert!(runtime.inspector.traces().iter().any(|e| e.kind == "capture" || e.kind == "target"));
}
