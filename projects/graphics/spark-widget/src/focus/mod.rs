//! 焦点与导航。

use crate::id::WidgetId;

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

    pub fn next(&mut self) {
        // TODO: Tab 顺序遍历。
    }

    pub fn previous(&mut self) {
        // TODO: Shift+Tab。
    }

    pub fn move_direction(&mut self, _dir: Direction) {
        // TODO: 方向键 / 手柄导航。
    }
}
