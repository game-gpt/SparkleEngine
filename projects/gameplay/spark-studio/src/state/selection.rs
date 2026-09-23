//! Hierarchy 演示数据与选中辅助。

use crate::project::ProjectKind;

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
