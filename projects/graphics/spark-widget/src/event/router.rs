//! 命中与事件分发（占位）。

use crate::runtime::{UiFrame, UiRuntime};

/// 将原始输入转为 UI 事件并路由。当前为占位。
pub fn dispatch(runtime: &mut UiRuntime, frame: &UiFrame<'_>) {
    let _ = &runtime.tree;
    let _ = frame.input;
    // TODO: hit test → capture / target / bubble。
    runtime.state.hovered = None;
}
