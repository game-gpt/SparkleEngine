//! 布局预设：默认 / 脚本 / 调试。

use super::DockLayoutState;

/// 编辑器布局预设。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutPreset {
    /// 默认：层级 + 场景 + 检查器 + 底栏。
    Default,
    /// 脚本：加宽中央，收窄侧栏。
    Script,
    /// 调试：加高底栏。
    Debug,
}

impl LayoutPreset {
    /// 工具栏显示名。
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "默认",
            Self::Script => "脚本",
            Self::Debug => "调试",
        }
    }

    /// 预设对应的停靠尺寸。
    pub fn dock_layout(self) -> DockLayoutState {
        match self {
            Self::Default => DockLayoutState::default(),
            Self::Script => DockLayoutState {
                hierarchy_width: 200.0,
                inspector_width: 260.0,
                bottom_height: 180.0,
                ..DockLayoutState::default()
            },
            Self::Debug => DockLayoutState {
                bottom_height: 280.0,
                ..DockLayoutState::default()
            },
        }
    }

    /// 应用预设尺寸，保留当前折叠态。
    pub fn apply_to(self, current: DockLayoutState) -> DockLayoutState {
        let next = self.dock_layout();
        DockLayoutState {
            hierarchy_width: next.hierarchy_width,
            inspector_width: next.inspector_width,
            bottom_height: next.bottom_height,
            hierarchy_collapsed: current.hierarchy_collapsed,
            inspector_collapsed: current.inspector_collapsed,
            bottom_collapsed: current.bottom_collapsed,
        }
    }
}
