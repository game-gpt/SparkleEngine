//! 命中与事件分发。

use spark_types::Vec2;
use spark_input::{Input, Key, MouseBtn};

use crate::{
    command::UiCommand,
    focus::{self, Direction},
    id::WidgetId,
    node::WidgetKind,
    runtime::{UiFrame, UiRuntime},
    tree::WidgetTree,
};

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
        dismiss_outside_overlays(runtime, hit);

        let capture = hit.filter(|id| consumes_pointer(&runtime.tree, *id));
        runtime.state.captured = capture;
        if let Some(id) = capture {
            if let Some(node) = runtime.tree.node_mut(id) {
                node.state.pressed = true;
            }
            if runtime.tree.node(id).map(|n| n.focusable).unwrap_or(false) {
                focus::set_focus(&mut runtime.tree, &mut runtime.focus, Some(id));
                crate::scroll::ensure_visible(&mut runtime.tree, id);
            }
            if runtime.tree.node(id).map(|n| n.content.drag_source).unwrap_or(false) {
                runtime.drag.begin_press(id, pos);
            }
            runtime.state.input_blocked = true;
        }
        else if hit.is_none() {
            focus::set_focus(&mut runtime.tree, &mut runtime.focus, None);
            runtime.drag.cancel();
        }
    }

    if input.mouse_down(MouseBtn::Left) {
        if let Some(id) = runtime.state.captured {
            if runtime.tree.node(id).map(|n| n.kind == WidgetKind::Slider).unwrap_or(false) {
                set_slider_value_at(&mut runtime.tree, id, pos.x);
            }
            let payload = runtime.tree.node(id).filter(|n| n.content.drag_source).map(|n| crate::drag_drop::DragPayload::new(n.id.raw()));
            if runtime.drag.update_move(pos, payload) {
                runtime.drag.hover_target = hit.filter(|t| runtime.tree.node(*t).map(|n| n.content.drop_target).unwrap_or(false));
            }
            runtime.state.input_blocked = true;
        }
        else if let Some(id) = hit {
            if consumes_pointer(&runtime.tree, id) {
                runtime.state.input_blocked = true;
            }
        }
    }

    if input.mouse_released(MouseBtn::Left) {
        let was_dragging = runtime.drag.is_dragging();
        if let Some((source, _payload, target)) = runtime.drag.end() {
            if let Some(target) = target {
                runtime.commands.push(UiCommand::Drop { source, target });
                runtime.inspector.push_trace("drop", Some(target), format!("source={}", source.raw()));
            }
            runtime.state.input_blocked = true;
            runtime.state.captured = None;
            clear_pressed(&mut runtime.tree);
        }
        else if !was_dragging {
            let captured = runtime.state.captured.take();
            let click_target = match (captured, hit) {
                (Some(c), Some(h)) if c == h => Some(c),
                (Some(c), _) => Some(c),
                (None, Some(h)) if consumes_pointer(&runtime.tree, h) => Some(h),
                _ => None,
            };
            if let Some(id) = click_target {
                if let Some(node) = runtime.tree.node_mut(id) {
                    node.state.pressed = false;
                }
                handle_click(runtime, id, pos);
                runtime.inspector.push_trace("click", Some(id), format!("pos=({:.1},{:.1})", pos.x, pos.y));
                runtime.state.input_blocked = true;
            }
            clear_pressed(&mut runtime.tree);
        }
        else {
            runtime.state.captured = None;
            clear_pressed(&mut runtime.tree);
        }
    }

    let wheel = input.wheel();
    if wheel.abs() > f32::EPSILON {
        let mut consumed = false;
        if let Some(target) = hit {
            for id in crate::event::bubble_path(&runtime.tree, target) {
                let is_scroll = runtime.tree.node(id).map(|n| matches!(n.kind, WidgetKind::ScrollView | WidgetKind::ListView)).unwrap_or(false);
                if is_scroll {
                    if let Some(node) = runtime.tree.node_mut(id) {
                        node.scroll.apply_wheel(wheel * 40.0);
                    }
                    runtime.inspector.push_trace("scroll", Some(id), format!("wheel={wheel:.2}"));
                    consumed = true;
                    break;
                }
            }
        }
        if consumed || modal.is_some() {
            runtime.state.input_blocked = true;
        }
        else if let Some(id) = hit {
            if consumes_pointer(&runtime.tree, id) {
                runtime.state.input_blocked = true;
            }
        }
    }

    dispatch_keys(runtime, input);

    let _ = PointerEvent { position: pos, target: hit };
}

fn dispatch_keys(runtime: &mut UiRuntime, input: &Input) {
    let shift = input.key_down(Key::LShift) || input.key_down(Key::RShift);
    let ctrl = input.key_down(Key::LCtrl) || input.key_down(Key::RCtrl);
    let trap = runtime.overlays.top_modal();
    if let Some(modal) = trap {
        focus::ensure_focus_in_trap(&runtime.tree, &mut runtime.focus, modal);
        sync_focus_flags(&mut runtime.tree, runtime.focus.focused);
    }

    let focused_is_field = runtime
        .focus
        .focused
        .and_then(|id| runtime.tree.node(id))
        .map(|n| matches!(n.kind, WidgetKind::TextField | WidgetKind::TextArea))
        .unwrap_or(false);

    let mut actions = Vec::new();
    let composing = focused_is_field && !input.composition().is_empty();
    if focused_is_field && !composing {
        if input.key_pressed(Key::Backspace) {
            actions.push(crate::text::TextEditAction::Backspace);
        }
        if input.key_pressed(Key::Delete) {
            actions.push(crate::text::TextEditAction::Delete);
        }
        if input.key_pressed(Key::Left) {
            actions.push(crate::text::TextEditAction::MoveLeft { select: shift });
        }
        if input.key_pressed(Key::Right) {
            actions.push(crate::text::TextEditAction::MoveRight { select: shift });
        }
        if input.key_pressed(Key::Home) {
            actions.push(crate::text::TextEditAction::Home { select: shift });
        }
        if input.key_pressed(Key::End) {
            actions.push(crate::text::TextEditAction::End { select: shift });
        }
        if ctrl && input.key_pressed(Key::A) {
            actions.push(crate::text::TextEditAction::SelectAll);
        }
        if ctrl && input.key_pressed(Key::C) {
            actions.push(crate::text::TextEditAction::Copy);
        }
        if ctrl && input.key_pressed(Key::X) {
            actions.push(crate::text::TextEditAction::Cut);
        }
        if ctrl && input.key_pressed(Key::V) {
            actions.push(crate::text::TextEditAction::Paste);
        }
    }

    if crate::text::apply_text_input(&mut runtime.tree, runtime.focus.focused, input.text(), &actions, Some(runtime.clipboard.as_mut())) {
        runtime.state.input_blocked = true;
        runtime.state.dirty.mark_layout();
    }
    else if focused_is_field
        && (ctrl && (input.key_pressed(Key::C) || input.key_pressed(Key::X) || input.key_pressed(Key::V) || input.key_pressed(Key::A)))
    {
        runtime.state.input_blocked = true;
    }

    sync_composition(runtime, input);
    if composing || !input.composition().is_empty() {
        runtime.state.input_blocked = true;
        runtime.state.dirty.mark_paint();
    }

    if input.key_pressed(Key::Tab) {
        if shift {
            focus::focus_previous_in(&runtime.tree, &mut runtime.focus, trap);
        }
        else {
            focus::focus_next_in(&runtime.tree, &mut runtime.focus, trap);
        }
        sync_focus_flags(&mut runtime.tree, runtime.focus.focused);
        if let Some(id) = runtime.focus.focused {
            crate::scroll::ensure_visible(&mut runtime.tree, id);
        }
        runtime.state.input_blocked = true;
    }

    // 文本框持有焦点时，方向键留给插入符，不抢导航。
    if !focused_is_field {
        if input.key_pressed(Key::Up) {
            focus::focus_direction_in(&runtime.tree, &mut runtime.focus, Direction::Up, trap);
            sync_focus_flags(&mut runtime.tree, runtime.focus.focused);
            if let Some(id) = runtime.focus.focused {
                crate::scroll::ensure_visible(&mut runtime.tree, id);
            }
            runtime.state.input_blocked = true;
        }
        if input.key_pressed(Key::Down) {
            focus::focus_direction_in(&runtime.tree, &mut runtime.focus, Direction::Down, trap);
            sync_focus_flags(&mut runtime.tree, runtime.focus.focused);
            if let Some(id) = runtime.focus.focused {
                crate::scroll::ensure_visible(&mut runtime.tree, id);
            }
            runtime.state.input_blocked = true;
        }
        if input.key_pressed(Key::Left) {
            focus::focus_direction_in(&runtime.tree, &mut runtime.focus, Direction::Left, trap);
            sync_focus_flags(&mut runtime.tree, runtime.focus.focused);
            if let Some(id) = runtime.focus.focused {
                crate::scroll::ensure_visible(&mut runtime.tree, id);
            }
            runtime.state.input_blocked = true;
        }
        if input.key_pressed(Key::Right) {
            focus::focus_direction_in(&runtime.tree, &mut runtime.focus, Direction::Right, trap);
            sync_focus_flags(&mut runtime.tree, runtime.focus.focused);
            if let Some(id) = runtime.focus.focused {
                crate::scroll::ensure_visible(&mut runtime.tree, id);
            }
            runtime.state.input_blocked = true;
        }

        if input.key_pressed(Key::Enter) || input.key_pressed(Key::Space) {
            if let Some(id) = runtime.focus.focused {
                handle_click(runtime, id, Vec2::ZERO);
                runtime.state.input_blocked = true;
            }
        }
    }

    if input.key_pressed(Key::Escape) {
        if runtime.drag.is_dragging() {
            runtime.drag.cancel();
            runtime.state.input_blocked = true;
        }
        else if let Some(entry) = runtime.overlays.pop_top() {
            runtime.tree.unmount(entry.id);
            runtime.state.input_blocked = true;
        }
        else {
            runtime.commands.push(UiCommand::CloseOverlay);
            runtime.state.input_blocked = true;
        }
    }
}

fn sync_composition(runtime: &mut UiRuntime, input: &Input) {
    let focused = runtime.focus.focused;
    let composition = input.composition().to_string();
    for id in runtime.tree.ids() {
        let Some(node) = runtime.tree.node_mut(id)
        else {
            continue;
        };
        if !matches!(node.kind, WidgetKind::TextField | WidgetKind::TextArea) {
            continue;
        }
        if Some(id) == focused {
            node.content.composition = composition.clone();
        }
        else if !node.content.composition.is_empty() {
            node.content.composition.clear();
        }
    }
}

fn handle_click(runtime: &mut UiRuntime, id: WidgetId, pos: Vec2) {
    use crate::response::EventResponse;

    let mut response = EventResponse::default();

    for wid in crate::event::capture_path(&runtime.tree, id) {
        let phase_resp = click_phase_response(&runtime.tree, wid);
        runtime.inspector.push_trace("capture", Some(wid), format!("click-target={}", id.raw()));
        response.merge(phase_resp);
        if response.stop_propagation {
            flush_event_commands(runtime, &mut response);
            return;
        }
    }

    {
        let phase_resp = click_phase_response(&runtime.tree, id);
        runtime.inspector.push_trace("target", Some(id), format!("pos=({:.1},{:.1})", pos.x, pos.y));
        response.merge(phase_resp);
    }

    if !response.prevent_default {
        apply_click_default(runtime, id, pos);
    }

    flush_event_commands(runtime, &mut response);
    if response.stop_propagation {
        let _ = ClickEvent { position: pos, target: Some(id) };
        return;
    }

    if let Some(parent) = runtime.tree.node(id).and_then(|n| n.parent) {
        for wid in crate::event::bubble_path(&runtime.tree, parent) {
            let phase_resp = click_phase_response(&runtime.tree, wid);
            runtime.inspector.push_trace("bubble", Some(wid), format!("click-from={}", id.raw()));
            response.merge(phase_resp);
            if response.stop_propagation {
                break;
            }
        }
    }

    flush_event_commands(runtime, &mut response);
    let _ = ClickEvent { position: pos, target: Some(id) };
}

fn click_phase_response(tree: &WidgetTree, id: WidgetId) -> crate::response::EventResponse {
    use crate::response::EventResponse;

    let Some(node) = tree.node(id)
    else {
        return EventResponse::default();
    };
    let mut resp = EventResponse::default();
    if node.content.prevent_click_default {
        resp.prevent_default = true;
        resp.handled = true;
    }
    if node.content.stop_click_propagation {
        resp.stop_propagation = true;
        resp.handled = true;
    }
    resp
}

fn flush_event_commands(runtime: &mut UiRuntime, response: &mut crate::response::EventResponse) {
    for command in response.commands.drain(..) {
        runtime.commands.push(command);
    }
}

fn apply_click_default(runtime: &mut UiRuntime, id: WidgetId, pos: Vec2) {
    if let Some(index) = crate::widgets::handle_tab_click(&mut runtime.tree, id) {
        runtime.inspector.push_trace("tab", Some(id), format!("selected={index}"));
    }

    let kind = runtime.tree.node(id).map(|n| n.kind);
    match kind {
        Some(WidgetKind::Checkbox) | Some(WidgetKind::Toggle) => {
            if let Some(node) = runtime.tree.node_mut(id) {
                let next = !node.content.checked;
                node.content.checked = next;
                node.state.checked = next;
            }
        }
        Some(WidgetKind::Radio) => {
            let parent = runtime.tree.node(id).and_then(|n| n.parent);
            if let Some(parent) = parent {
                let siblings = runtime.tree.node(parent).map(|n| n.children.clone()).unwrap_or_default();
                for sibling in siblings {
                    if let Some(node) = runtime.tree.node_mut(sibling) {
                        if node.kind == WidgetKind::Radio {
                            let on = sibling == id;
                            node.content.checked = on;
                            node.state.checked = on;
                        }
                    }
                }
            }
            else if let Some(node) = runtime.tree.node_mut(id) {
                node.content.checked = true;
                node.state.checked = true;
            }
        }
        Some(WidgetKind::Slider) => {
            set_slider_value_at(&mut runtime.tree, id, pos.x);
        }
        _ => {}
    }

    let command = runtime.tree.node(id).and_then(|n| n.content.click_command.clone());
    if let Some(command) = command {
        runtime.commands.push(command);
    }
}

fn set_slider_value_at(tree: &mut WidgetTree, id: WidgetId, x: f32) {
    let Some(node) = tree.node(id)
    else {
        return;
    };
    if node.kind != WidgetKind::Slider {
        return;
    }
    let rect = node.computed.content_rect;
    let t = if rect.w <= f32::EPSILON { 0.0 } else { ((x - rect.x) / rect.w).clamp(0.0, 1.0) };
    let min = node.content.value_min;
    let max = node.content.value_max;
    if let Some(node) = tree.node_mut(id) {
        node.content.value = min + (max - min) * t;
    }
}

/// 自上而下命中：后挂载的兄弟优先（更靠上）。
pub fn hit_test(tree: &WidgetTree, id: WidgetId, point: Vec2) -> Option<WidgetId> {
    let Some(node) = tree.node(id)
    else {
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
        if matches!(node.kind, WidgetKind::ScrollView | WidgetKind::ListView) && !clip.contains(point) {
            return None;
        }
    }

    for child in node.children.iter().rev() {
        if let Some(clip) = node.computed.clip_rect {
            if matches!(node.kind, WidgetKind::ScrollView | WidgetKind::ListView) && !clip.contains(point) {
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
    }
    else if node.kind == WidgetKind::Root {
        None
    }
    else if node.layer == crate::runtime::UiLayer::Hud {
        // HUD 非交互区域（标签、面板空白）不吞命中，让世界输入通过。
        None
    }
    else if node.children.is_empty() {
        Some(id)
    }
    else {
        // GUI 容器空白区仍算命中（阻断世界，便于面板遮挡）。
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

/// 指针事件是否应由该节点消费（从而阻断世界）。
fn consumes_pointer(tree: &WidgetTree, id: WidgetId) -> bool {
    let Some(node) = tree.node(id)
    else {
        return false;
    };
    match node.layer {
        crate::runtime::UiLayer::Hud => is_hittable_leaf(node.kind) || node.focusable,
        crate::runtime::UiLayer::Gui | crate::runtime::UiLayer::Overlay => {
            is_hittable_leaf(node.kind)
                || node.focusable
                || matches!(node.kind, WidgetKind::Modal | WidgetKind::Popup | WidgetKind::Panel | WidgetKind::Container)
        }
    }
}

fn blocks_world_input(tree: &WidgetTree, id: WidgetId) -> bool {
    consumes_pointer(tree, id)
}

fn dismiss_outside_overlays(runtime: &mut UiRuntime, hit: Option<WidgetId>) {
    let to_close: Vec<WidgetId> = runtime
        .overlays
        .iter()
        .filter(|e| e.dismiss_on_outside)
        .filter(|e| match hit {
            None => true,
            Some(h) => !is_descendant_or_self(&runtime.tree, e.id, h),
        })
        .map(|e| e.id)
        .collect();
    for id in to_close {
        runtime.overlays.remove(id);
        runtime.tree.unmount(id);
    }
}

fn is_descendant_or_self(tree: &WidgetTree, ancestor: WidgetId, mut id: WidgetId) -> bool {
    loop {
        if id == ancestor {
            return true;
        }
        let Some(parent) = tree.node(id).and_then(|n| n.parent)
        else {
            return false;
        };
        id = parent;
    }
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
