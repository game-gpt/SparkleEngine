//! 停靠布局状态（持久化至 `.spark/studio-layout.json`）。

use serde::{Deserialize, Serialize};

/// 可调整的停靠尺寸与折叠态。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
            hierarchy_width: 224.0,
            inspector_width: 320.0,
            bottom_height: 220.0,
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
    /// 分栏线视觉宽度。
    pub const SPLITTER_THICKNESS: f32 = 1.0;
    /// 分栏指针命中半宽。
    pub const SPLITTER_HIT_HALF: f32 = 3.0;

    /// 层级面板最小宽度。
    pub const MIN_HIERARCHY_WIDTH: f32 = 180.0;
    /// 层级面板最大宽度。
    pub const MAX_HIERARCHY_WIDTH: f32 = 420.0;
    /// 检查器最小宽度。
    pub const MIN_INSPECTOR_WIDTH: f32 = 240.0;
    /// 检查器最大宽度。
    pub const MAX_INSPECTOR_WIDTH: f32 = 480.0;
    /// 底栏最小高度。
    pub const MIN_BOTTOM_HEIGHT: f32 = 120.0;
    /// 中央场景最小宽度。
    pub const MIN_SCENE_WIDTH: f32 = 420.0;
    /// 中央场景最小高度。
    pub const MIN_SCENE_HEIGHT: f32 = 240.0;

    /// 顶栏与状态栏占用后的主工作区纵向起点。
    pub fn main_top(self) -> f32 {
        Self::MENU_HEIGHT + Self::TOOLBAR_HEIGHT + Self::SPLITTER_THICKNESS
    }

    /// 主工作区高度（不含状态栏）。
    pub fn main_height(self, screen_h: f32) -> f32 {
        (screen_h - self.main_top() - Self::STATUS_HEIGHT).max(0.0)
    }

    /// 层级有效宽度（折叠时为 0）。
    pub fn hierarchy_effective(self) -> f32 {
        if self.hierarchy_collapsed { 0.0 } else { self.hierarchy_width }
    }

    /// 检查器有效宽度（折叠时为 0）。
    pub fn inspector_effective(self) -> f32 {
        if self.inspector_collapsed { 0.0 } else { self.inspector_width }
    }

    /// 底栏有效高度（折叠时为 0）。
    pub fn bottom_effective(self) -> f32 {
        if self.bottom_collapsed { 0.0 } else { self.bottom_height }
    }

    /// 约束层级宽度。
    pub fn clamp_hierarchy(self, width: f32, screen_w: f32) -> f32 {
        let max = (screen_w - self.inspector_effective() - Self::MIN_SCENE_WIDTH - Self::SPLITTER_THICKNESS * 2.0).max(Self::MIN_HIERARCHY_WIDTH);
        width.clamp(Self::MIN_HIERARCHY_WIDTH, max.min(Self::MAX_HIERARCHY_WIDTH))
    }

    /// 约束检查器宽度。
    pub fn clamp_inspector(self, width: f32, screen_w: f32) -> f32 {
        let max = (screen_w - self.hierarchy_effective() - Self::MIN_SCENE_WIDTH - Self::SPLITTER_THICKNESS * 2.0).max(Self::MIN_INSPECTOR_WIDTH);
        width.clamp(Self::MIN_INSPECTOR_WIDTH, max.min(Self::MAX_INSPECTOR_WIDTH))
    }

    /// 约束底栏高度。
    pub fn clamp_bottom(self, height: f32, screen_h: f32) -> f32 {
        let main_h = self.main_height(screen_h);
        let max = (main_h - Self::MIN_SCENE_HEIGHT - Self::SPLITTER_THICKNESS).max(Self::MIN_BOTTOM_HEIGHT);
        let cap = main_h * 0.45;
        height.clamp(Self::MIN_BOTTOM_HEIGHT, max.min(cap))
    }
}
