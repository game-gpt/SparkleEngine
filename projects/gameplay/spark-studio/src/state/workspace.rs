//! Studio 会话 UI 状态。

use super::{BottomTab, CenterTab, PlayMode, Tool};

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
            status: String::new(),
        }
    }
}
