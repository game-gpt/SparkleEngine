//! Studio 会话 UI 状态。

use crate::layout::{DockLayoutState, LayoutPreset};

use super::{BottomTab, CenterTab, PlayMode, Tool, TransformState, ViewportState};

/// Studio 会话 UI 状态（与 Widget 树命令互通）。
#[derive(Debug, Clone)]
pub struct EditorState {
    /// Play 状态机。
    pub play: PlayMode,
    /// 中央标签。
    pub center: CenterTab,
    /// 底部标签。
    pub bottom: BottomTab,
    /// 当前工具。
    pub tool: Tool,
    /// Hierarchy 选中实体 ID。
    pub selected: u64,
    /// 停靠布局尺寸与折叠态。
    pub dock: DockLayoutState,
    /// 当前布局预设。
    pub layout_preset: LayoutPreset,
    /// 检查器 Transform 演示字段。
    pub transform: TransformState,
    /// 场景视口相机。
    pub viewport: ViewportState,
    /// 状态栏短文案。
    pub status: String,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            play: PlayMode::Edit,
            center: CenterTab::Scene,
            bottom: BottomTab::Project,
            tool: Tool::Move,
            selected: 1,
            dock: DockLayoutState::default(),
            layout_preset: LayoutPreset::Default,
            transform: TransformState::default(),
            viewport: ViewportState::default(),
            status: String::new(),
        }
    }
}
