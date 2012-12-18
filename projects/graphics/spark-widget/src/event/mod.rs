//! UI 事件与路由。

pub mod bubble;
pub mod router;

pub use bubble::{Phase, bubble_from, bubble_path, capture_path, propagate};
pub use router::hit_test;

use crate::id::WidgetId;

#[derive(Debug, Clone)]
pub enum UiEvent {
    PointerMove(PointerEvent),
    PointerDown(PointerEvent),
    PointerUp(PointerEvent),
    Click(ClickEvent),
    Scroll(ScrollEvent),
    KeyDown(KeyEvent),
    KeyUp(KeyEvent),
    TextInput(TextInputEvent),
    FocusIn,
    FocusOut,
}

#[derive(Debug, Clone)]
pub struct PointerEvent {
    pub position: spark_core::Vec2,
    pub target: Option<WidgetId>,
}

#[derive(Debug, Clone)]
pub struct ClickEvent {
    pub position: spark_core::Vec2,
    pub target: Option<WidgetId>,
}

#[derive(Debug, Clone)]
pub struct ScrollEvent {
    pub delta: spark_core::Vec2,
    pub target: Option<WidgetId>,
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub key: spark_input::Key,
}

#[derive(Debug, Clone)]
pub struct TextInputEvent {
    pub text: String,
}

/// 事件处理上下文。
pub struct EventContext<'a> {
    pub tree: &'a mut crate::tree::WidgetTree,
    pub focus: &'a mut crate::focus::FocusManager,
    pub overlays: &'a mut crate::overlay::OverlayManager,
    pub commands: &'a mut crate::command::UiCommandQueue,
    pub phase: Phase,
}
