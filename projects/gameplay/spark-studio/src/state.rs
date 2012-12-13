//! 编辑器演示状态（场景实体 / 选中 / Play 模式）。

use crate::project::ProjectKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayMode {
    Edit,
    Play,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterTab {
    Scene,
    Game,
    Script,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    Project,
    Console,
    Problems,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Hand,
    Move,
    Rotate,
    Scale,
}

#[derive(Debug, Clone)]
pub struct EntityRow {
    pub id: u64,
    pub name: &'static str,
    pub depth: u8,
    pub component_summary: &'static str,
}

#[derive(Debug, Clone)]
pub struct EditorState {
    pub play: PlayMode,
    pub center: CenterTab,
    pub bottom: BottomTab,
    pub tool: Tool,
    pub selected: u64,
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

pub fn hierarchy_for(kind: ProjectKind) -> &'static [EntityRow] {
    match kind {
        ProjectKind::Rust => PING_PONG,
        ProjectKind::Valkyrie => SNAKE,
        ProjectKind::Hybrid => TETRIS,
    }
}

pub fn entity_by_id(kind: ProjectKind, id: u64) -> Option<&'static EntityRow> {
    hierarchy_for(kind).iter().find(|e| e.id == id)
}

pub fn default_selected(kind: ProjectKind) -> u64 {
    hierarchy_for(kind).get(1).map(|e| e.id).unwrap_or(1)
}

const PING_PONG: &[EntityRow] = &[
    EntityRow {
        id: 1,
        name: "MainScene",
        depth: 0,
        component_summary: "Scene",
    },
    EntityRow {
        id: 2,
        name: "Camera",
        depth: 1,
        component_summary: "Transform, Camera",
    },
    EntityRow {
        id: 3,
        name: "Table",
        depth: 1,
        component_summary: "Transform, SpriteRenderer",
    },
    EntityRow {
        id: 4,
        name: "LeftPaddle",
        depth: 1,
        component_summary: "Transform, Paddle (Rust)",
    },
    EntityRow {
        id: 5,
        name: "RightPaddle",
        depth: 1,
        component_summary: "Transform, Paddle (Rust)",
    },
    EntityRow {
        id: 6,
        name: "Ball",
        depth: 1,
        component_summary: "Transform, Ball (Rust)",
    },
];

const SNAKE: &[EntityRow] = &[
    EntityRow {
        id: 1,
        name: "MainScene",
        depth: 0,
        component_summary: "Scene",
    },
    EntityRow {
        id: 2,
        name: "Camera",
        depth: 1,
        component_summary: "Transform, Camera",
    },
    EntityRow {
        id: 3,
        name: "Board",
        depth: 1,
        component_summary: "Transform",
    },
    EntityRow {
        id: 4,
        name: "Snake",
        depth: 1,
        component_summary: "Transform, SnakeController (Valkyrie)",
    },
    EntityRow {
        id: 5,
        name: "Food",
        depth: 1,
        component_summary: "Transform, Food (Valkyrie)",
    },
    EntityRow {
        id: 6,
        name: "HUD",
        depth: 1,
        component_summary: "WidgetRoot, Hud (Valkyrie)",
    },
];

const TETRIS: &[EntityRow] = &[
    EntityRow {
        id: 1,
        name: "MainScene",
        depth: 0,
        component_summary: "Scene",
    },
    EntityRow {
        id: 2,
        name: "Camera",
        depth: 1,
        component_summary: "Transform, Camera",
    },
    EntityRow {
        id: 3,
        name: "Board",
        depth: 1,
        component_summary: "Transform, Board (Rust)",
    },
    EntityRow {
        id: 4,
        name: "ActivePiece",
        depth: 1,
        component_summary: "Transform, Piece (Rust)",
    },
    EntityRow {
        id: 5,
        name: "HUD",
        depth: 1,
        component_summary: "WidgetRoot, Hud (Valkyrie)",
    },
];

pub const CMD_FILE_SAVE: u64 = 100;
pub const CMD_EDIT_UNDO: u64 = 110;
pub const CMD_PLAY: u64 = 200;
pub const CMD_PAUSE: u64 = 201;
pub const CMD_STEP: u64 = 202;
pub const CMD_STOP: u64 = 203;
pub const CMD_TOOL_HAND: u64 = 300;
pub const CMD_TOOL_MOVE: u64 = 301;
pub const CMD_TOOL_ROTATE: u64 = 302;
pub const CMD_TOOL_SCALE: u64 = 303;
pub const CMD_TAB_SCENE: u64 = 400;
pub const CMD_TAB_GAME: u64 = 401;
pub const CMD_TAB_SCRIPT: u64 = 402;
pub const CMD_BOTTOM_PROJECT: u64 = 410;
pub const CMD_BOTTOM_CONSOLE: u64 = 411;
pub const CMD_BOTTOM_PROBLEMS: u64 = 412;
pub const CMD_WINDOW_GALLERY: u64 = 500;
pub const CMD_SELECT_BASE: u64 = 1000;

pub fn select_cmd(entity_id: u64) -> u64 {
    CMD_SELECT_BASE + entity_id
}

pub fn parse_select_cmd(cmd: u64) -> Option<u64> {
    if cmd >= CMD_SELECT_BASE {
        Some(cmd - CMD_SELECT_BASE)
    } else {
        None
    }
}
