//! 焦点与导航。

use crate::id::WidgetId;
use crate::tree::WidgetTree;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Default)]
pub struct Neighbors {
    pub up: Option<WidgetId>,
    pub down: Option<WidgetId>,
    pub left: Option<WidgetId>,
    pub right: Option<WidgetId>,
}

#[derive(Debug, Clone)]
pub struct FocusPolicy {
    pub focusable: bool,
    pub tab_index: i32,
    pub neighbors: Neighbors,
}

impl Default for FocusPolicy {
    fn default() -> Self {
        Self {
            focusable: false,
            tab_index: 0,
            neighbors: Neighbors::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct FocusManager {
    pub focused: Option<WidgetId>,
}

impl FocusManager {
    pub fn request(&mut self, id: WidgetId) {
        self.focused = Some(id);
    }

    pub fn clear(&mut self) {
        self.focused = None;
    }
}

/// 收集深度优先、可聚焦且未禁用的节点。
pub fn collect_focusable(tree: &WidgetTree) -> Vec<WidgetId> {
    let mut out = Vec::new();
    collect_focusable_rec(tree, tree.root(), &mut out);
    out
}

fn collect_focusable_rec(tree: &WidgetTree, id: WidgetId, out: &mut Vec<WidgetId>) {
    let Some(node) = tree.node(id) else {
        return;
    };
    if !node.state.visible || node.state.disabled {
        return;
    }
    if node.focusable {
        out.push(id);
    }
    for child in &node.children {
        collect_focusable_rec(tree, *child, out);
    }
}

pub fn set_focus(tree: &mut WidgetTree, focus: &mut FocusManager, id: Option<WidgetId>) {
    focus.focused = id;
    for node_id in tree.ids() {
        if let Some(node) = tree.node_mut(node_id) {
            node.state.focused = Some(node_id) == id;
        }
    }
}

pub fn focus_next(tree: &WidgetTree, focus: &mut FocusManager) {
    let list = collect_focusable(tree);
    if list.is_empty() {
        focus.focused = None;
        return;
    }
    let next = match focus.focused.and_then(|cur| list.iter().position(|id| *id == cur)) {
        Some(index) => list[(index + 1) % list.len()],
        None => list[0],
    };
    focus.focused = Some(next);
}

pub fn focus_previous(tree: &WidgetTree, focus: &mut FocusManager) {
    let list = collect_focusable(tree);
    if list.is_empty() {
        focus.focused = None;
        return;
    }
    let prev = match focus.focused.and_then(|cur| list.iter().position(|id| *id == cur)) {
        Some(index) => list[(index + list.len() - 1) % list.len()],
        None => list[list.len() - 1],
    };
    focus.focused = Some(prev);
}

pub fn focus_direction(tree: &WidgetTree, focus: &mut FocusManager, dir: Direction) {
    let list = collect_focusable(tree);
    if list.is_empty() {
        return;
    }

    // 显式邻居优先。
    if let Some(current) = focus.focused {
        if let Some(node) = tree.node(current) {
            let neighbor = match dir {
                Direction::Up => node.neighbors.up,
                Direction::Down => node.neighbors.down,
                Direction::Left => node.neighbors.left,
                Direction::Right => node.neighbors.right,
            };
            if let Some(id) = neighbor {
                if tree.node(id).map(|n| n.focusable && !n.state.disabled).unwrap_or(false) {
                    focus.focused = Some(id);
                    return;
                }
            }
        }
    }

    let Some(current) = focus.focused.or_else(|| list.first().copied()) else {
        return;
    };
    let Some(cur_rect) = tree.node(current).map(|n| n.computed.rect) else {
        return;
    };
    let cur_c = cur_rect.center();

    let mut best: Option<(WidgetId, f32)> = None;
    for id in list {
        if id == current {
            continue;
        }
        let Some(rect) = tree.node(id).map(|n| n.computed.rect) else {
            continue;
        };
        let c = rect.center();
        let dx = c.x - cur_c.x;
        let dy = c.y - cur_c.y;
        let ok = match dir {
            Direction::Up => dy < -1.0 && dx.abs() <= dy.abs() + rect.w,
            Direction::Down => dy > 1.0 && dx.abs() <= dy.abs() + rect.w,
            Direction::Left => dx < -1.0 && dy.abs() <= dx.abs() + rect.h,
            Direction::Right => dx > 1.0 && dy.abs() <= dx.abs() + rect.h,
        };
        if !ok {
            continue;
        }
        let dist = dx * dx + dy * dy;
        if best.map(|(_, d)| dist < d).unwrap_or(true) {
            best = Some((id, dist));
        }
    }
    if let Some((id, _)) = best {
        focus.focused = Some(id);
    } else {
        match dir {
            Direction::Down | Direction::Right => focus_next(tree, focus),
            Direction::Up | Direction::Left => focus_previous(tree, focus),
        }
    }
}
