//! 跨帧 UI 运行时状态（非单个 Widget 局部态）。

use crate::id::WidgetId;

#[derive(Debug, Default)]
pub struct UiState {
    /// 指针当前悬停目标。
    pub hovered: Option<WidgetId>,
    /// 指针捕获目标。
    pub captured: Option<WidgetId>,
    /// 本帧是否有 UI 消耗了输入（阻断世界）。
    pub input_blocked: bool,
}
