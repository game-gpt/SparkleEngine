//! Studio 会话 UI 状态。

use crate::layout::{DockLayoutState, LayoutPreset};

use super::{BottomTab, CenterTab, ConsoleLog, PlayMode, Problem, ProblemSeverity, Tool, TransformState, ViewportState};

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
    /// 项目面板选中的资源行索引。
    pub selected_asset: Option<u32>,
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
    /// 控制台日志。
    pub console: ConsoleLog,
    /// 用户 UI 缩放（叠在 DPI 之上）。
    pub ui_scale: f32,
    /// Play / 脚本等运行时诊断。
    pub runtime_problems: Vec<Problem>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            play: PlayMode::Edit,
            center: CenterTab::Scene,
            bottom: BottomTab::Project,
            tool: Tool::Move,
            selected: 1,
            selected_asset: None,
            dock: DockLayoutState::default(),
            layout_preset: LayoutPreset::Default,
            transform: TransformState::default(),
            viewport: ViewportState::default(),
            status: String::new(),
            console: ConsoleLog::default(),
            ui_scale: 1.0,
            runtime_problems: Vec::new(),
        }
    }
}

impl EditorState {
    /// 写入状态栏并追加控制台行。
    pub fn log(&mut self, line: impl Into<String>) {
        let line = line.into();
        self.console.push(&line);
        self.status = line;
    }

    /// 记录运行时问题并写入控制台。
    pub fn note_runtime_problem(&mut self, severity: ProblemSeverity, message: impl Into<String>) {
        let message = message.into();
        self.runtime_problems.push(Problem { severity, message: message.clone() });
        self.console.push(format!("{} {}", severity.glyph(), message));
    }

    /// 清除运行时问题（成功启动 Play 等）。
    pub fn clear_runtime_problems(&mut self) {
        self.runtime_problems.clear();
    }
}
