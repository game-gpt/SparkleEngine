//! 编辑器演示状态（场景实体 / 选中 / Play 模式）。

use crate::project::ProjectKind;

/// 工具栏 Play 状态机。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayMode {
    /// 编辑：不跑游戏主循环。
    Edit,
    /// 播放：进程内嵌入示例 `GameHost`。
    Play,
    /// 暂停：保留会话，不再 `update`。
    Paused,
}

/// 中央工作区标签。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterTab {
    /// 场景视图（编辑器视口）。
    Scene,
    /// 游戏视图（Play 画面）。
    Game,
    /// 脚本 / 检视辅助页。
    Script,
}

/// 底部面板标签。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    /// 项目资源树。
    Project,
    /// 控制台日志。
    Console,
    /// 诊断 / Problems。
    Problems,
}

/// 场景操作工具（与工具栏互斥）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// 平移视口。
    Hand,
    /// 平移选中实体。
    Move,
    /// 旋转选中实体。
    Rotate,
    /// 缩放选中实体。
    Scale,
}

/// Hierarchy 一行：演示用静态实体描述（非 ECS 权威）。
#[derive(Debug, Clone)]
pub struct EntityRow {
    /// 稳定演示 ID（命令路由用）。
    pub id: u64,
    /// 显示名。
    pub name: &'static str,
    /// 树缩进层级（0=根）。
    pub depth: u8,
    /// Inspector 摘要文案（组件列表示意）。
    pub component_summary: &'static str,
}

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

/// 按项目种类返回演示 Hierarchy 表。
pub fn hierarchy_for(kind: ProjectKind) -> &'static [EntityRow] {
    match kind {
        ProjectKind::Rust => PING_PONG,
        ProjectKind::Valkyrie => SNAKE,
        ProjectKind::Hybrid => TETRIS,
    }
}

/// 在当前 Hierarchy 中按 ID 查找行。
pub fn entity_by_id(kind: ProjectKind, id: u64) -> Option<&'static EntityRow> {
    hierarchy_for(kind).iter().find(|e| e.id == id)
}

/// 打开项目时的默认选中：优先第二行（常见为 Camera），否则 `1`。
pub fn default_selected(kind: ProjectKind) -> u64 {
    hierarchy_for(kind).get(1).map(|e| e.id).unwrap_or(1)
}

const PING_PONG: &[EntityRow] = &[
    EntityRow { id: 1, name: "MainScene", depth: 0, component_summary: "Scene" },
    EntityRow { id: 2, name: "Camera", depth: 1, component_summary: "Transform, Camera" },
    EntityRow { id: 3, name: "Table", depth: 1, component_summary: "Transform, SpriteRenderer" },
    EntityRow { id: 4, name: "LeftPaddle", depth: 1, component_summary: "Transform, Paddle (Rust)" },
    EntityRow { id: 5, name: "RightPaddle", depth: 1, component_summary: "Transform, Paddle (Rust)" },
    EntityRow { id: 6, name: "Ball", depth: 1, component_summary: "Transform, Ball (Rust)" },
];

const SNAKE: &[EntityRow] = &[
    EntityRow { id: 1, name: "MainScene", depth: 0, component_summary: "Scene" },
    EntityRow { id: 2, name: "Camera", depth: 1, component_summary: "Transform, Camera" },
    EntityRow { id: 3, name: "Board", depth: 1, component_summary: "Transform" },
    EntityRow { id: 4, name: "Snake", depth: 1, component_summary: "Transform, SnakeController (Valkyrie)" },
    EntityRow { id: 5, name: "Food", depth: 1, component_summary: "Transform, Food (Valkyrie)" },
    EntityRow { id: 6, name: "HUD", depth: 1, component_summary: "WidgetRoot, Hud (Valkyrie)" },
];

const TETRIS: &[EntityRow] = &[
    EntityRow { id: 1, name: "MainScene", depth: 0, component_summary: "Scene" },
    EntityRow { id: 2, name: "Camera", depth: 1, component_summary: "Transform, Camera" },
    EntityRow { id: 3, name: "Board", depth: 1, component_summary: "Transform, Board (Rust)" },
    EntityRow { id: 4, name: "ActivePiece", depth: 1, component_summary: "Transform, Piece (Rust)" },
    EntityRow { id: 5, name: "HUD", depth: 1, component_summary: "WidgetRoot, Hud (Valkyrie)" },
];

/// 菜单命令：保存。
pub const CMD_FILE_SAVE: u64 = 100;
/// 菜单命令：撤销。
pub const CMD_EDIT_UNDO: u64 = 110;
/// 工具栏：Play。
pub const CMD_PLAY: u64 = 200;
/// 工具栏：Pause。
pub const CMD_PAUSE: u64 = 201;
/// 工具栏：单步。
pub const CMD_STEP: u64 = 202;
/// 工具栏：Stop（回 Edit）。
pub const CMD_STOP: u64 = 203;
/// 工具：Hand。
pub const CMD_TOOL_HAND: u64 = 300;
/// 工具：Move。
pub const CMD_TOOL_MOVE: u64 = 301;
/// 工具：Rotate。
pub const CMD_TOOL_ROTATE: u64 = 302;
/// 工具：Scale。
pub const CMD_TOOL_SCALE: u64 = 303;
/// 中央标签：Scene。
pub const CMD_TAB_SCENE: u64 = 400;
/// 中央标签：Game。
pub const CMD_TAB_GAME: u64 = 401;
/// 中央标签：Script。
pub const CMD_TAB_SCRIPT: u64 = 402;
/// 底部标签：Project。
pub const CMD_BOTTOM_PROJECT: u64 = 410;
/// 底部标签：Console。
pub const CMD_BOTTOM_CONSOLE: u64 = 411;
/// 底部标签：Problems。
pub const CMD_BOTTOM_PROBLEMS: u64 = 412;
/// 窗口：控件图鉴。
pub const CMD_WINDOW_GALLERY: u64 = 500;
/// Hierarchy 选中命令基址：实际命令 = 基址 + 实体 ID。
pub const CMD_SELECT_BASE: u64 = 1000;

/// 编码「选中实体」命令 ID。
pub fn select_cmd(entity_id: u64) -> u64 {
    CMD_SELECT_BASE + entity_id
}

/// 解析选中命令；非选中区间返回 `None`。
pub fn parse_select_cmd(cmd: u64) -> Option<u64> {
    if cmd >= CMD_SELECT_BASE { Some(cmd - CMD_SELECT_BASE) } else { None }
}
