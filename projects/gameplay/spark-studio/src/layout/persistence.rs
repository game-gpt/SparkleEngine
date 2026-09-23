//! 停靠布局状态（后续按项目持久化）。

/// 可调整的停靠尺寸与折叠态。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockLayoutState {
    /// 层级面板宽度。
    pub hierarchy_width: f32,
    /// 检查器面板宽度。
    pub inspector_width: f32,
    /// 底栏高度。
    pub bottom_height: f32,
    /// 层级面板是否折叠。
    pub hierarchy_collapsed: bool,
    /// 检查器面板是否折叠。
    pub inspector_collapsed: bool,
    /// 底栏是否折叠。
    pub bottom_collapsed: bool,
}

impl Default for DockLayoutState {
    fn default() -> Self {
        Self {
            hierarchy_width: 240.0,
            inspector_width: 300.0,
            bottom_height: 180.0,
            hierarchy_collapsed: false,
            inspector_collapsed: false,
            bottom_collapsed: false,
        }
    }
}

impl DockLayoutState {
    /// 面板标题行高度。
    pub const HEADER_HEIGHT: f32 = 24.0;
    /// 顶栏菜单高度。
    pub const MENU_HEIGHT: f32 = 24.0;
    /// 场景工具栏高度。
    pub const TOOLBAR_HEIGHT: f32 = 32.0;
    /// 状态栏高度。
    pub const STATUS_HEIGHT: f32 = 20.0;
    /// 页签条高度。
    pub const TAB_HEIGHT: f32 = 24.0;
}
