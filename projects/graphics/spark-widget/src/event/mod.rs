//! UI 事件与路由。

pub mod bubble;
pub mod router;

pub use bubble::{Phase, bubble_from, bubble_path, capture_path, propagate};
pub use router::hit_test;

use crate::id::WidgetId;

/// 进入 Widget 树的输入 / 焦点事件。
#[derive(Debug, Clone)]
pub enum UiEvent {
    /// 指针移动。
    PointerMove(PointerEvent),
    /// 指针按下。
    PointerDown(PointerEvent),
    /// 指针抬起。
    PointerUp(PointerEvent),
    /// 完整点击（按下后在同一目标抬起）。
    Click(ClickEvent),
    /// 滚轮 / 触控板滚动。
    Scroll(ScrollEvent),
    /// 键盘按下。
    KeyDown(KeyEvent),
    /// 键盘抬起。
    KeyUp(KeyEvent),
    /// 文本输入（含 IME 提交）。
    TextInput(TextInputEvent),
    /// 焦点进入当前控件。
    FocusIn,
    /// 焦点离开当前控件。
    FocusOut,
}

/// 指针位置与命中目标。
#[derive(Debug, Clone)]
pub struct PointerEvent {
    /// 屏幕坐标。
    pub position: spark_types::Vec2,
    /// 命中测试得到的控件；未命中为 `None`。
    pub target: Option<WidgetId>,
}

/// 点击位置与目标。
#[derive(Debug, Clone)]
pub struct ClickEvent {
    /// 点击屏幕坐标。
    pub position: spark_types::Vec2,
    /// 点击目标控件。
    pub target: Option<WidgetId>,
}

/// 滚动增量与起始目标。
#[derive(Debug, Clone)]
pub struct ScrollEvent {
    /// 滚动量（逻辑像素；正负号依平台约定）。
    pub delta: spark_types::Vec2,
    /// 滚轮位于的控件（再向上找 ScrollView）。
    pub target: Option<WidgetId>,
}

/// 键盘按键事件。
#[derive(Debug, Clone)]
pub struct KeyEvent {
    /// 物理/逻辑键。
    pub key: spark_input::Key,
}

/// 文本输入载荷。
#[derive(Debug, Clone)]
pub struct TextInputEvent {
    /// 本次提交的 UTF-8 文本。
    pub text: String,
}

/// 事件处理上下文。
pub struct EventContext<'a> {
    /// 可变控件树。
    pub tree: &'a mut crate::tree::WidgetTree,
    /// 焦点管理器。
    pub focus: &'a mut crate::focus::FocusManager,
    /// Overlay 栈。
    pub overlays: &'a mut crate::overlay::OverlayManager,
    /// 本帧命令队列。
    pub commands: &'a mut crate::command::UiCommandQueue,
    /// 当前传播阶段（capture / target / bubble）。
    pub phase: Phase,
}
