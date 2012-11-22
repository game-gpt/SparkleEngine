//! Retained Widget 系统。
//!
//! 公共 API 为长期持有的 [`WidgetTree`] 与 [`UiRuntime`]。
//! 每帧经 layout / event / update 后，由 paint traversal 写入 [`spark_renderer::DrawList`]。
//!
//! 界面控件叫 **Widget**，避免与 ECS `Component` 混淆。
//! UI 动效在 [`motion`]，不属于 `spark-animator`。
//!
//! 不提供立即模式控件公共 API。

pub mod accessibility;
pub mod command;
pub mod drag_drop;
pub mod event;
pub mod focus;
pub mod id;
pub mod inspector;
pub mod layout;
pub mod motion;
pub mod node;
pub mod overlay;
pub mod paint;
pub mod response;
pub mod runtime;
pub mod scroll;
pub mod state;
pub mod style;
pub mod text;
pub mod tree;
pub mod widgets;

pub use accessibility::{AccessibilityNode, AccessibilityTree, Role};
pub use command::{UiCommand, UiCommandQueue};
pub use drag_drop::{DragPayload, DragState};
pub use event::{hit_test, EventContext, UiEvent};
pub use focus::{Direction, FocusManager, FocusPolicy, Neighbors};
pub use id::WidgetId;
pub use inspector::UiInspector;
pub use layout::{
    Align, Constraints, FlexDirection, Insets, Justify, Layout, LayoutSpec, Size,
};
pub use motion::{Easing, MotionManager, SpringConfig, Transition};
pub use node::{WidgetContent, WidgetKind, WidgetNode, WidgetStateFlags};
pub use overlay::{OverlayLayer, OverlayManager};
pub use paint::PaintContext;
pub use response::EventResponse;
pub use runtime::{UiFrame, UiLayer, UiRuntime};
pub use state::UiState;
pub use style::{ComputedStyle, Style, Theme};
pub use scroll::{ensure_visible, find_scroll_ancestor, ScrollDirection, ScrollState};
pub use text::{apply_text_input, measure_plain, TextLayout, TextStyle};
pub use tree::WidgetTree;
pub use widgets::{
    button_widget, checkbox_widget, column, hud_root, label_widget, modal_widget, overlay_root,
    panel, progress_widget, radio_widget, row, scroll_view, separator_widget, slider_widget,
    spacer_widget, text_field_widget, toggle_widget, WidgetBuilder,
};
