//! 焦点与导航。

use crate::{id::WidgetId, tree::WidgetTree};

/// 方向键 / 手柄导航的四个方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// 向上。
    Up,
    /// 向下。
    Down,
    /// 向左。
    Left,
    /// 向右。
    Right,
}

/// 节点上显式声明的方向键邻居（优先于几何寻焦）。
#[derive(Debug, Clone, Default)]
pub struct Neighbors {
    /// 上方向邻居。
    pub up: Option<WidgetId>,
    /// 下方向邻居。
    pub down: Option<WidgetId>,
    /// 左方向邻居。
    pub left: Option<WidgetId>,
    /// 右方向邻居。
    pub right: Option<WidgetId>,
}

/// 挂到节点上的完整焦点策略。
#[derive(Debug, Clone)]
pub struct FocusPolicy {
    /// 是否可聚焦。
    pub focusable: bool,
    /// Tab 序：`>0` 升序优先，`0` 跟文档序，`<0` 可点聚焦但不进 Tab 环。
    pub tab_index: i32,
    /// 方向键显式邻居。
    pub neighbors: Neighbors,
}

impl Default for FocusPolicy {
    fn default() -> Self {
        Self { focusable: false, tab_index: 0, neighbors: Neighbors::default() }
    }
}

/// 当前焦点持有者。
#[derive(Debug, Default)]
pub struct FocusManager {
    /// 当前聚焦的控件；无焦点为 `None`。
    pub focused: Option<WidgetId>,
}

impl FocusManager {
    /// 请求将焦点移到 `id`（不写回节点伪态，需再调 [`set_focus`]）。
    pub fn request(&mut self, id: WidgetId) {
        self.focused = Some(id);
    }

    /// 清除焦点持有者。
    pub fn clear(&mut self) {
        self.focused = None;
    }
}

/// 收集可聚焦且未禁用的节点，并按 `tab_index` 排序（用于 Tab 环）。
pub fn collect_focusable(tree: &WidgetTree) -> Vec<WidgetId> {
    collect_focusable_in(tree, tree.root())
}

/// 在子树内收集可聚焦节点（用于 modal focus trap）。
///
/// 顺序：`tab_index > 0` 升序，其后 `tab_index == 0` 按文档序；`tab_index < 0` 不进 Tab 环。
pub fn collect_focusable_in(tree: &WidgetTree, root: WidgetId) -> Vec<WidgetId> {
    let mut positive: Vec<(i32, usize, WidgetId)> = Vec::new();
    let mut zero: Vec<WidgetId> = Vec::new();
    let mut doc_order = 0usize;
    collect_focusable_rec(tree, root, &mut positive, &mut zero, &mut doc_order);
    positive.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let mut out: Vec<WidgetId> = positive.into_iter().map(|(_, _, id)| id).collect();
    out.extend(zero);
    out
}

fn collect_focusable_rec(
    tree: &WidgetTree,
    id: WidgetId,
    positive: &mut Vec<(i32, usize, WidgetId)>,
    zero: &mut Vec<WidgetId>,
    doc_order: &mut usize,
) {
    let Some(node) = tree.node(id)
    else {
        return;
    };
    if !node.state.visible || node.state.disabled {
        return;
    }
    if node.focusable {
        let order = *doc_order;
        *doc_order += 1;
        if node.tab_index > 0 {
            positive.push((node.tab_index, order, id));
        }
        else if node.tab_index == 0 {
            zero.push(id);
        }
        // tab_index < 0：可聚焦但不参与 Tab 环。
    }
    for child in &node.children {
        collect_focusable_rec(tree, *child, positive, zero, doc_order);
    }
}

/// 若当前焦点不在 `trap_root` 子树内，则落到该子树第一个可聚焦节点。
pub fn ensure_focus_in_trap(tree: &WidgetTree, focus: &mut FocusManager, trap_root: WidgetId) {
    let list = collect_focusable_in(tree, trap_root);
    if list.is_empty() {
        focus.focused = None;
        return;
    }
    let inside = focus.focused.map(|id| is_descendant_or_self(tree, trap_root, id)).unwrap_or(false);
    if !inside {
        focus.focused = Some(list[0]);
    }
}

fn is_descendant_or_self(tree: &WidgetTree, root: WidgetId, id: WidgetId) -> bool {
    let mut cur = Some(id);
    while let Some(c) = cur {
        if c == root {
            return true;
        }
        cur = tree.node(c).and_then(|n| n.parent);
    }
    false
}

/// 设置焦点并同步各节点 `state.focused` 伪态。
pub fn set_focus(tree: &mut WidgetTree, focus: &mut FocusManager, id: Option<WidgetId>) {
    focus.focused = id;
    for node_id in tree.ids() {
        if let Some(node) = tree.node_mut(node_id) {
            node.state.focused = Some(node_id) == id;
        }
    }
}

fn focusable_list(tree: &WidgetTree, trap: Option<WidgetId>) -> Vec<WidgetId> {
    match trap {
        Some(root) => collect_focusable_in(tree, root),
        None => collect_focusable(tree),
    }
}

/// 全树 Tab 环前进一格。
pub fn focus_next(tree: &WidgetTree, focus: &mut FocusManager) {
    focus_next_in(tree, focus, None);
}

/// 全树 Tab 环后退一格。
pub fn focus_previous(tree: &WidgetTree, focus: &mut FocusManager) {
    focus_previous_in(tree, focus, None);
}

/// 在可选 focus trap 内 Tab 前进；`trap = None` 表示全树。
pub fn focus_next_in(tree: &WidgetTree, focus: &mut FocusManager, trap: Option<WidgetId>) {
    let list = focusable_list(tree, trap);
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

/// 在可选 focus trap 内 Tab 后退。
pub fn focus_previous_in(tree: &WidgetTree, focus: &mut FocusManager, trap: Option<WidgetId>) {
    let list = focusable_list(tree, trap);
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

/// 全树按方向键寻焦（显式邻居优先，否则几何最近）。
pub fn focus_direction(tree: &WidgetTree, focus: &mut FocusManager, dir: Direction) {
    focus_direction_in(tree, focus, dir, None);
}

/// 在可选 trap 内按方向寻焦；无候选时回退到 Tab 前后。
pub fn focus_direction_in(tree: &WidgetTree, focus: &mut FocusManager, dir: Direction, trap: Option<WidgetId>) {
    let list = focusable_list(tree, trap);
    if list.is_empty() {
        return;
    }

    // 显式邻居优先（仍须落在 trap 内）。
    if let Some(current) = focus.focused {
        if let Some(node) = tree.node(current) {
            let neighbor = match dir {
                Direction::Up => node.neighbors.up,
                Direction::Down => node.neighbors.down,
                Direction::Left => node.neighbors.left,
                Direction::Right => node.neighbors.right,
            };
            if let Some(id) = neighbor {
                let allowed = match trap {
                    Some(root) => is_descendant_or_self(tree, root, id),
                    None => true,
                };
                if allowed && tree.node(id).map(|n| n.focusable && !n.state.disabled).unwrap_or(false) {
                    focus.focused = Some(id);
                    return;
                }
            }
        }
    }

    let Some(current) = focus.focused.or_else(|| list.first().copied())
    else {
        return;
    };
    let Some(cur_rect) = tree.node(current).map(|n| n.computed.rect)
    else {
        return;
    };
    let cur_c = cur_rect.center();

    let mut best: Option<(WidgetId, f32)> = None;
    for id in &list {
        if *id == current {
            continue;
        }
        let Some(rect) = tree.node(*id).map(|n| n.computed.rect)
        else {
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
            best = Some((*id, dist));
        }
    }
    if let Some((id, _)) = best {
        focus.focused = Some(id);
    }
    else {
        match dir {
            Direction::Down | Direction::Right => focus_next_in(tree, focus, trap),
            Direction::Up | Direction::Left => focus_previous_in(tree, focus, trap),
        }
    }
}
