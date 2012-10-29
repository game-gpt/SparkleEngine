//! 跨帧 UI 状态：热/活动/焦点/捕获与控件记忆。

use std::collections::HashMap;

use spark_core::{Rect, Vec2};

use crate::accessibility::AccessTree;
use crate::debug::UiDebug;
use crate::drag_drop::DragState;
use crate::id::WidgetId;
use crate::motion::MotionScheduler;
use crate::overlay::OverlayState;
use crate::prefs::UiPrefs;

/// 焦点来源。导航与环绘制可据此区分。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusSource {
    #[default]
    Programmatic,
    Pointer,
    Keyboard,
    Gamepad,
}

/// 单个控件跨帧记忆。
#[derive(Debug, Clone, Default)]
pub struct WidgetMemory {
    pub opened: bool,
    pub selected: bool,
    pub scroll: Vec2,
    pub last_rect: Option<Rect>,
    pub f32_value: f32,
    /// 文本光标（按字符计）。
    pub cursor: usize,
}

/// 所有跨帧状态。由游戏持有，每帧借给 [`crate::Ui`]。
#[derive(Debug, Default)]
pub struct UiState {
    pub hot: Option<WidgetId>,
    pub active: Option<WidgetId>,
    pub focused: Option<WidgetId>,
    pub focus_source: FocusSource,
    /// 指针捕获（拖动滑条等）。被捕获时命中测试优先给它。
    pub captured: Option<WidgetId>,
    pub memory: HashMap<WidgetId, WidgetMemory>,
    pub motion: MotionScheduler,
    pub overlays: OverlayState,
    pub drag: DragState,
    pub access: AccessTree,
    pub debug: UiDebug,
    pub prefs: UiPrefs,
    /// 本帧登记的可聚焦顺序（Tab）及矩形（方向键）。
    pub(crate) focus_order: Vec<WidgetId>,
    /// Modal 打开时，Tab 与方向键只在这些 ID 之间移动。
    pub(crate) focus_trap: Option<Vec<WidgetId>>,
    pub(crate) focus_rects: HashMap<WidgetId, Rect>,
    /// 本帧重复 ID 检测。
    pub(crate) seen_ids: HashMap<WidgetId, u32>,
    frame: u64,
}

impl UiState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn memory(&self, id: WidgetId) -> Option<&WidgetMemory> {
        self.memory.get(&id)
    }

    pub fn memory_mut(&mut self, id: WidgetId) -> &mut WidgetMemory {
        self.memory.entry(id).or_default()
    }

    pub fn request_focus(&mut self, id: WidgetId, source: FocusSource) {
        self.focused = Some(id);
        self.focus_source = source;
    }

    pub fn clear_focus(&mut self) {
        self.focused = None;
    }

    pub(crate) fn begin_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        self.hot = None;
        self.focus_order.clear();
        self.focus_trap = None;
        self.focus_rects.clear();
        self.seen_ids.clear();
        self.overlays.begin_frame();
        self.drag.begin_frame();
        self.access.clear();
    }

    pub(crate) fn note_id(&mut self, id: WidgetId) {
        *self.seen_ids.entry(id).or_insert(0) += 1;
    }

    pub(crate) fn register_focusable(&mut self, id: WidgetId, rect: Rect) {
        self.focus_order.push(id);
        self.focus_rects.insert(id, rect);
    }

    pub fn id_conflicts(&self) -> Vec<WidgetId> {
        self.seen_ids
            .iter()
            .filter_map(|(id, count)| (*count > 1).then_some(*id))
            .collect()
    }
}
