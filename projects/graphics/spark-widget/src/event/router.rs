//! 命中与事件分发。

use spark_core::Vec2;
use spark_input::{Input, Key, MouseBtn};

use crate::command::UiCommand;
use crate::focus::{self, Direction};
use crate::id::WidgetId;
use crate::node::WidgetKind;
use crate::runtime::{UiFrame, UiRuntime};
use crate::tree::WidgetTree;

use super::{ClickEvent, PointerEvent};

/// 将原始输入转为 UI 事件并路由。
pub fn dispatch(runtime: &mut UiRuntime, frame: &UiFrame<'_>) {
    let input = frame.input;
    let (mx, my) = input.mouse_pos();
    let pos = Vec2::new(mx, my);

    clear_transient_hover(&mut runtime.tree);

    let modal = runtime.overlays.top_modal();
    let hit_root = modal.unwrap_or_else(|| runtime.tree.root());
    let hit = hit_test(&runtime.tree, hit_root, pos);
    runtime.state.hovered = hit;

    if modal.is_some() {
        runtime.state.input_blocked = true;
    }

    if let Some(id) = hit {
        if let Some(node) = runtime.tree.node_mut(id) {
            node.state.hovered = true;
        }
        if blocks_world_input(&runtime.tree, id) {
            runtime.state.input_blocked = true;
        }
    }

    if input.mouse_pressed(MouseBtn::Left) {
        runtime.state.captured = hit;
        if let Some(id) = hit {
            if let Some(node) = runtime.tree.node_mut(id) {
                node.state.pressed = true;
            }
            if runtime.tree.node(id).map(|n| n.focusable).unwrap_or(false) {
                focus::set_focus(&mut runtime.tree, &mut runtime.focus, Some(id));
            }
            runtime.state.input_blocked = true;
        } else {
            focus::set_focus(&mut runtime.tree, &mut runtime.focus, None);
        }
    }

    if input.mouse_released(MouseBtn::Left) {
        let captured = runtime.state.captured.take();
        let click_target = match (captured, hit) {
            (Some(c), Some(h)) if c == h => Some(c),
            (Some(c), _) => Some(c),
            _ => hit,
        };
        if let Some(id) = click_target {
            if let Some(node) = runtime.tree.node_mut(id) {
                node.state.pressed = false;
            }
            handle_click(runtime, id, pos);
            runtime.state.input_blocked = true;
        }
        clear_pressed(&mut runtime.tree);
    }

    if input.mouse_down(MouseBtn::Left) {
        if let Some(id) = runtime.state.captured {
            if runtime
                .tree
                .node(id)
                .map(|n| n.kind == WidgetKind::Slider)
                .unwrap_or(false)
            {
                set_slider_value_at(&mut runtime.tree, id, pos.x);
            }
            runtime.state.input_blocked = true;
        } else if hit.is_some() {
            runtime.state.input_blocked = true;
        }
    }

    let wheel = input.wheel();
    if wheel.abs() > f32::EPSILON {
        let scroll_id = hit.and_then(|id| crate::scroll::find_scroll_ancestor(&runtime.tree, id));
        if let Some(id) = scroll_id {
            if let Some(node) = runtime.tree.node_mut(id) {
                node.scroll.apply_wheel(wheel * 40.0);
            }
            runtime.state.input_blocked = true;
        } else if hit.is_some() || modal.is_some() {
            runtime.state.input_blocked = true;
        }
    }

    dispatch_keys(runtime, input);

    let _ = PointerEvent {
        position: pos,
        target: hit,
    };
}

fn dispatch_keys(runtime: &mut UiRuntime, input: &Input) {
    let shift = input.key_down(Key::LShift) || input.key_down(Key::RShift);

    if input.key_pressed(Key::Tab) {
        if shift {
            focus::focus_previous(&runtime.tree, &mut runtime.focus);
        } else {
            focus::focus_next(&runtime.tree, &mut runtime.focus);
        }
        sync_focus_flags(&mut runtime.tree, runtime.focus.focused);
        runtime.state.input_blocked = true;
    }

    if input.key_pressed(Key::Up) {
        focus::focus_direction(&runtime.tree, &mut runtime.focus, Direction::Up);
        sync_focus_flags(&mut runtime.tree, runtime.focus.focused);
        runtime.state.input_blocked = true;
    }
    if input.key_pressed(Key::Down) {
        focus::focus_direction(&runtime.tree, &mut runtime.focus, Direction::Down);
        sync_focus_flags(&mut runtime.tree, runtime.focus.focused);
        runtime.state.input_blocked = true;
    }
    if input.key_pressed(Key::Left) {
        focus::focus_direction(&runtime.tree, &mut runtime.focus, Direction::Left);
        sync_focus_flags(&mut runtime.tree, runtime.focus.focused);
        runtime.state.input_blocked = true;
    }
    if input.key_pressed(Key::Right) {
        focus::focus_direction(&runtime.tree, &mut runtime.focus, Direction::Right);
        sync_focus_flags(&mut runtime.tree, runtime.focus.focused);
        runtime.state.input_blocked = true;
    }

    if input.key_pressed(Key::Enter) || input.key_pressed(Key::Space) {
        if let Some(id) = runtime.focus.focused {
            handle_click(runtime, id, Vec2::ZERO);
            runtime.state.input_blocked = true;
        }
    }

    if input.key_pressed(Key::Escape) {
        if let Some(entry) = runtime.overlays.pop_top() {
            runtime.tree.unmount(entry.id);
        } else {
            runtime.commands.push(UiCommand::CloseOverlay);
        }
        runtime.state.input_blocked = true;
    }
}

fn handle_click(runtime: &mut UiRuntime, id: WidgetId, pos: Vec2) {
    let kind = runtime.tree.node(id).map(|n| n.kind);
    match kind {
        Some(WidgetKind::Checkbox) | Some(WidgetKind::Toggle) => {
            if let Some(node) = runtime.tree.node_mut(id) {
                let next = !node.content.checked;
                node.content.checked = next;
                node.state.checked = next;
            }
        }
        Some(WidgetKind::Slider) => {
            set_slider_value_at(&mut runtime.tree, id, pos.x);
        }
        _ => {}
    }

    let command = runtime
        .tree
        .node(id)
        .and_then(|n| n.content.click_command.clone());
    if let Some(command) = command {
        runtime.commands.push(command);
    }
    let _ = ClickEvent {
        position: pos,
        target: Some(id),
    };
}

fn set_slider_value_at(tree: &mut WidgetTree, id: WidgetId, x: f32) {
    let Some(node) = tree.node(id) else {
        return;
    };
    if node.kind != WidgetKind::Slider {
        return;
    }
    let rect = node.computed.content_rect;
    let t = if rect.w <= f32::EPSILON {
        0.0
    } else {
        ((x - rect.x) / rect.w).clamp(0.0, 1.0)
    };
    let min = node.content.value_min;
    let max = node.content.value_max;
    if let Some(node) = tree.node_mut(id) {
        node.content.value = min + (max - min) * t;
    }
}

/// 自上而下命中：后挂载的兄弟优先（更靠上）。
pub fn hit_test(tree: &WidgetTree, id: WidgetId, point: Vec2) -> Option<WidgetId> {
    let Some(node) = tree.node(id) else {
        return None;
    };
    if !node.state.visible || node.state.disabled {
        return None;
    }
    if !node.computed.rect.contains(point) {
        return None;
    }
    // ScrollView 等裁剪区域外的子节点不可命中。
    if let Some(clip) = node.computed.clip_rect {
        if node.kind == WidgetKind::ScrollView && !clip.contains(point) {
            return None;
        }
    }

    for child in node.children.iter().rev() {
        if let Some(clip) = node.computed.clip_rect {
            if node.kind == WidgetKind::ScrollView && !clip.contains(point) {
                continue;
            }
        }
        if let Some(hit) = hit_test(tree, *child, point) {
            return Some(hit);
        }
    }

    // 容器默认穿透到子级；无子命中时，可命中自身（按钮等）。
    if is_hittable_leaf(node.kind) || node.focusable {
        Some(id)
    } else if node.kind == WidgetKind::Root {
        None
    } else if node.children.is_empty() {
        Some(id)
    } else {
        // 点在容器空白区：仍算命中容器（可阻断 HUD 穿透策略由 layer 决定）。
        Some(id)
    }
}

fn is_hittable_leaf(kind: WidgetKind) -> bool {
    matches!(
        kind,
        WidgetKind::Button
            | WidgetKind::Toggle
            | WidgetKind::Checkbox
            | WidgetKind::Radio
            | WidgetKind::Slider
            | WidgetKind::TextField
            | WidgetKind::TextArea
            | WidgetKind::ScrollView
            | WidgetKind::ListView
            | WidgetKind::TabView
            | WidgetKind::Modal
            | WidgetKind::Popup
    )
}

fn blocks_world_input(tree: &WidgetTree, id: WidgetId) -> bool {
    tree.node(id)
        .map(|n| is_hittable_leaf(n.kind) || n.focusable || n.kind == WidgetKind::Modal)
        .unwrap_or(false)
}

fn clear_transient_hover(tree: &mut WidgetTree) {
    for id in tree.ids() {
        if let Some(node) = tree.node_mut(id) {
            node.state.hovered = false;
        }
    }
}

fn clear_pressed(tree: &mut WidgetTree) {
    for id in tree.ids() {
        if let Some(node) = tree.node_mut(id) {
            node.state.pressed = false;
        }
    }
}

fn sync_focus_flags(tree: &mut WidgetTree, focused: Option<WidgetId>) {
    for id in tree.ids() {
        if let Some(node) = tree.node_mut(id) {
            node.state.focused = Some(id) == focused;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{run_layout, LayoutSpec, Size};
    use crate::widgets::{button_widget, checkbox_widget, column};
    use spark_input::ButtonState;

    fn frame<'a>(input: &'a Input, w: f32, h: f32) -> UiFrame<'a> {
        UiFrame {
            dt: 1.0 / 60.0,
            screen_size: Vec2::new(w, h),
            dpi_scale: 1.0,
            input,
        }
    }

    #[test]
    fn hit_test_prefers_topmost_child() {
        let mut tree = WidgetTree::new();
        let root = tree.root();
        // 根下绝对叠放两个按钮，后挂载者命中优先。
        let panel = tree.mount(root, crate::node::WidgetKind::Container).unwrap();
        if let Some(node) = tree.node_mut(panel) {
            node.layout = LayoutSpec {
                kind: crate::layout::Layout::Absolute,
                width: Size::Fill,
                height: Size::Fill,
                ..LayoutSpec::default()
            };
        }
        let a = button_widget()
            .key("a")
            .text("A")
            .layout(LayoutSpec {
                width: Size::Px(100.0),
                height: Size::Px(40.0),
                ..LayoutSpec::default()
            })
            .mount(&mut tree, panel)
            .unwrap();
        let b = button_widget()
            .key("b")
            .text("B")
            .layout(LayoutSpec {
                width: Size::Px(100.0),
                height: Size::Px(40.0),
                ..LayoutSpec::default()
            })
            .mount(&mut tree, panel)
            .unwrap();
        run_layout(&mut tree, Vec2::new(200.0, 200.0), 1.0);
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
            .layout(LayoutSpec {
                width: Size::Px(80.0),
                height: Size::Px(40.0),
                ..LayoutSpec::default()
            })
            .mount(&mut runtime.tree, root)
            .unwrap();
        run_layout(&mut runtime.tree, Vec2::new(200.0, 200.0), 1.0);

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
            .layout(LayoutSpec {
                width: Size::Px(120.0),
                height: Size::Px(28.0),
                ..LayoutSpec::default()
            })
            .mount(&mut runtime.tree, root)
            .unwrap();
        run_layout(&mut runtime.tree, Vec2::new(200.0, 200.0), 1.0);
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
        use crate::overlay::OverlayLayer;
        use crate::widgets::modal_widget;

        let mut runtime = UiRuntime::new();
        let root = runtime.tree.root();
        button_widget()
            .text("Behind")
            .on_click(UiCommand::Custom(1))
            .layout(LayoutSpec {
                width: Size::Px(80.0),
                height: Size::Px(40.0),
                ..LayoutSpec::default()
            })
            .mount(&mut runtime.tree, root)
            .unwrap();
        runtime
            .open_overlay(
                OverlayLayer::Modal,
                modal_widget().child(
                    button_widget()
                        .text("Modal")
                        .on_click(UiCommand::Custom(2))
                        .layout(LayoutSpec {
                            width: Size::Px(100.0),
                            height: Size::Px(40.0),
                            ..LayoutSpec::default()
                        }),
                ),
            )
            .unwrap();
        run_layout(&mut runtime.tree, Vec2::new(400.0, 300.0), 1.0);

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
        assert!(
            runtime.drain_commands().next().is_none(),
            "clicks under modal must not reach behind button"
        );
        assert!(runtime.state.input_blocked);

        input.begin_frame();
        input.on_key(Key::Escape, ButtonState::Pressed);
        let f = frame(&input, 400.0, 300.0);
        runtime.dispatch_input(&f);
        assert!(runtime.overlays.is_empty());
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
        run_layout(&mut runtime.tree, Vec2::new(200.0, 200.0), 1.0);

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
}
