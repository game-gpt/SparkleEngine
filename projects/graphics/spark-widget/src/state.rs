//! 跨帧 UI 运行时状态（非单个 Widget 局部态）。

use crate::id::WidgetId;

/// 失效标记：避免每帧无条件全树 layout / paint。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiDirty {
    /// 需要重新跑布局（通常连带重绘）。
    pub layout: bool,
    /// 需要重新遍历绘制。
    pub paint: bool,
    /// 样式 / 主题变更后需重算 computed style。
    pub style: bool,
}

impl Default for UiDirty {
    fn default() -> Self {
        // 首帧必须跑一遍。
        Self { layout: true, paint: true, style: true }
    }
}

impl UiDirty {
    /// 标记布局脏；同时置位 paint，因几何变化必然影响绘制。
    pub fn mark_layout(&mut self) {
        self.layout = true;
        self.paint = true;
    }

    /// 仅标记需要重绘（几何未变、视觉伪态变化等）。
    pub fn mark_paint(&mut self) {
        self.paint = true;
    }

    /// 标记样式脏；同时置位 paint。
    pub fn mark_style(&mut self) {
        self.style = true;
        self.paint = true;
    }

    /// 布局完成后清除 layout 脏位。
    pub fn clear_layout(&mut self) {
        self.layout = false;
    }

    /// 绘制完成后清除 paint 脏位。
    pub fn clear_paint(&mut self) {
        self.paint = false;
    }
}

/// 整棵 Widget 树共用的交互与脏状态（悬停、捕获、输入阻断等）。
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
