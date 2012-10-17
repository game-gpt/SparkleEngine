//! 方向焦点导航：在已登记的可聚焦矩形间选最近目标。

use spark_core::{Rect, Vec2};

use crate::id::WidgetId;
use crate::layout::Direction;
use crate::state::{FocusSource, UiState};

pub(crate) fn focus_in_direction(state: &mut UiState, direction: Direction, forward: bool) {
    let Some(current) = state.focused else {
        if let Some(first) = state.focus_order.first().copied() {
            state.request_focus(first, FocusSource::Keyboard);
        }
        return;
    };
    let Some(from) = state.focus_rects.get(&current).copied() else {
        return;
    };
    let origin = from.center();
    let mut best: Option<(WidgetId, f32)> = None;
    for id in &state.focus_order {
        if *id == current {
            continue;
        }
        let Some(rect) = state.focus_rects.get(id).copied() else {
            continue;
        };
        let target = rect.center();
        if !in_half_plane(origin, target, direction, forward) {
            continue;
        }
        let score = focus_score(origin, target, direction);
        if best.map(|(_, s)| score < s).unwrap_or(true) {
            best = Some((*id, score));
        }
    }
    if let Some((id, _)) = best {
        state.request_focus(id, FocusSource::Keyboard);
    }
}

fn in_half_plane(origin: Vec2, target: Vec2, direction: Direction, forward: bool) -> bool {
    let dx = target.x - origin.x;
    let dy = target.y - origin.y;
    match (direction, forward) {
        (Direction::Horizontal, true) => dx > 1.0,
        (Direction::Horizontal, false) => dx < -1.0,
        (Direction::Vertical, true) => dy > 1.0,
        (Direction::Vertical, false) => dy < -1.0,
    }
}

fn focus_score(origin: Vec2, target: Vec2, direction: Direction) -> f32 {
    let dx = target.x - origin.x;
    let dy = target.y - origin.y;
    match direction {
        Direction::Horizontal => dx.abs() + dy.abs() * 2.0,
        Direction::Vertical => dy.abs() + dx.abs() * 2.0,
    }
}

#[allow(dead_code)]
pub(crate) fn rect_distance(a: Rect, b: Rect) -> f32 {
    let ac = a.center();
    let bc = b.center();
    let dx = ac.x - bc.x;
    let dy = ac.y - bc.y;
    (dx * dx + dy * dy).sqrt()
}
