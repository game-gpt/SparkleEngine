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
pub use event::{
    bubble_from, bubble_path, capture_path, hit_test, propagate, EventContext, Phase, UiEvent,
};
pub use focus::{
    collect_focusable, collect_focusable_in, ensure_focus_in_trap, focus_direction,
    focus_direction_in, focus_next, focus_next_in, focus_previous, focus_previous_in, set_focus,
    Direction, FocusManager, FocusPolicy, Neighbors,
};
pub use id::WidgetId;
pub use inspector::{LayoutDump, UiEventTrace, UiInspector};
pub use layout::{
    Align, Constraints, FlexDirection, Insets, Justify, Layout, LayoutSpec, Size,
};
pub use motion::{Easing, MotionManager, MotionSample, SpringConfig, StyleProperty, Transition};
pub use node::{WidgetContent, WidgetKind, WidgetNode, WidgetStateFlags};
pub use overlay::{OverlayEntry, OverlayLayer, OverlayManager};
pub use paint::PaintContext;
pub use response::EventResponse;
pub use runtime::{UiFrame, UiLayer, UiRuntime};
pub use state::UiState;
pub use style::{ComputedStyle, Style, Theme};
pub use scroll::{ensure_visible, find_scroll_ancestor, ScrollDirection, ScrollState};
pub use text::{
    apply_text_input, measure_plain, Clipboard, EstimateMeasurer, FontMeasurer, MemoryClipboard,
    TextEditAction, TextLayout, TextMeasurer, TextStyle,
};
pub use tree::WidgetTree;
pub use widgets::{
    button_widget, checkbox_widget, column, content_height, grid, handle_tab_click, hud_root,
    label_widget, list_view, modal_widget, overlay_root, panel, popup_widget, progress_widget,
    radio_widget, row, scroll_view, separator_widget, slider_widget, spacer_widget, sync_tabs,
    sync_visible_rows, tab_view, text_field_widget, toast_widget, toggle_widget, tooltip_widget,
    visible_row_range, WidgetBuilder,
};
