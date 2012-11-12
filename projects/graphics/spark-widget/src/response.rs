//! 控件交互结果。比单独的 `bool` 多出悬停、焦点与变更信息。

use spark_core::Rect;

use crate::id::WidgetId;

/// 本帧对一个控件的交互摘要。
#[derive(Debug, Clone, Copy)]
pub struct Response {
    pub id: WidgetId,
    pub rect: Rect,
    pub hovered: bool,
    pub active: bool,
    pub focused: bool,
    pub clicked: bool,
    pub changed: bool,
    pub double_clicked: bool,
    pub long_pressed: bool,
    pub hover_entered: bool,
    pub hover_left: bool,
}

impl Response {
    pub fn empty(id: WidgetId, rect: Rect) -> Self {
        Self {
            id,
            rect,
            hovered: false,
            active: false,
            focused: false,
            clicked: false,
            changed: false,
            double_clicked: false,
            long_pressed: false,
            hover_entered: false,
            hover_left: false,
        }
    }

    pub fn clicked(self) -> bool {
        self.clicked
    }

    pub fn hovered(self) -> bool {
        self.hovered
    }

    pub fn active(self) -> bool {
        self.active
    }

    pub fn focused(self) -> bool {
        self.focused
    }

    pub fn changed(self) -> bool {
        self.changed
    }

    pub fn double_clicked(self) -> bool {
        self.double_clicked
    }

    pub fn long_pressed(self) -> bool {
        self.long_pressed
    }

    pub fn hover_entered(self) -> bool {
        self.hover_entered
    }

    pub fn hover_left(self) -> bool {
        self.hover_left
    }

    pub fn with_changed(mut self, changed: bool) -> Self {
        self.changed = changed;
        self
    }
}
