//! 跨帧 UI 运行时状态（非单个 Widget 局部态）。

use crate::id::WidgetId;

/// 失效标记：避免每帧无条件全树 layout / paint。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiDirty {
    pub layout: bool,
    pub paint: bool,
    pub style: bool,
}

impl Default for UiDirty {
    fn default() -> Self {
        // 首帧必须跑一遍。
        Self {
            layout: true,
            paint: true,
            style: true,
        }
    }
}

impl UiDirty {
    pub fn mark_layout(&mut self) {
        self.layout = true;
        self.paint = true;
    }

    pub fn mark_paint(&mut self) {
        self.paint = true;
    }

    pub fn mark_style(&mut self) {
        self.style = true;
        self.paint = true;
    }

    pub fn clear_layout(&mut self) {
        self.layout = false;
    }

    pub fn clear_paint(&mut self) {
        self.paint = false;
    }
}

#[derive(Debug, Default)]
pub struct UiState {
    /// 指针当前悬停目标。
    pub hovered: Option<WidgetId>,
    /// 指针捕获目标。
    pub captured: Option<WidgetId>,
    /// 本帧是否有 UI 消耗了输入（阻断世界）。
    pub input_blocked: bool,
    /// layout / paint / style 脏标记。
    pub dirty: UiDirty,
}
