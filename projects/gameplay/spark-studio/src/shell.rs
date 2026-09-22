//! Unity Default 停靠布局。
//!
//! ```text
//! Menu
//! Toolbar（工具左 · Play 中 · Layout 右）
//! ┌──────────┬─────────────────────┬───────────┐
//! │ Hierarchy│ Scene / Game        │ Inspector │
//! │          ├─────────────────────┤           │
//! │          │ Project / Console   │           │
//! └──────────┴─────────────────────┴───────────┘
//! Status
//! ```
//! Project 只落在中栏下方，不横跨 Hierarchy / Inspector。

use spark_types::Color;
use spark_widget::{
    Insets, Justify, LayoutSpec, Size, Style, UiCommand, WidgetBuilder, button_widget, column, label_widget, panel, row, spacer_widget,
};

use crate::{
    project::{ProjectInfo, ProjectKind},
    state::{
        BottomTab, CMD_BOTTOM_CONSOLE, CMD_BOTTOM_PROBLEMS, CMD_BOTTOM_PROJECT, CMD_EDIT_UNDO, CMD_FILE_SAVE, CMD_PAUSE, CMD_PLAY, CMD_STEP,
        CMD_STOP, CMD_TAB_GAME, CMD_TAB_SCENE, CMD_TAB_SCRIPT, CMD_TOOL_HAND, CMD_TOOL_MOVE, CMD_TOOL_ROTATE, CMD_TOOL_SCALE,
        CMD_WINDOW_GALLERY, CenterTab, EditorState, PlayMode, Tool, entity_by_id, hierarchy_for, select_cmd,
    },
};

const HIERARCHY_W: f32 = 240.0;
const INSPECTOR_W: f32 = 300.0;
const BOTTOM_H: f32 = 180.0;
const MENU_H: f32 = 22.0;
const TOOLBAR_H: f32 = 30.0;
const STATUS_H: f32 = 18.0;
const TAB_H: f32 = 22.0;
const HEADER_H: f32 = 20.0;

mod theme {
    use spark_types::Color;

    pub fn bg() -> Color {
        Color::rgb(0.15, 0.15, 0.15)
    }
    pub fn menu() -> Color {
        Color::rgb(0.18, 0.18, 0.18)
    }
    pub fn toolbar() -> Color {
        Color::rgb(0.20, 0.20, 0.20)
    }
    pub fn panel() -> Color {
        Color::rgb(0.22, 0.22, 0.22)
    }
    pub fn panel_header() -> Color {
        Color::rgb(0.24, 0.24, 0.24)
    }
    pub fn viewport() -> Color {
        Color::rgb(0.11, 0.11, 0.11)
    }
    pub fn status() -> Color {
        Color::rgb(0.13, 0.13, 0.13)
    }
    pub fn text() -> Color {
        Color::rgb(0.88, 0.88, 0.88)
    }
    pub fn text_dim() -> Color {
        Color::rgb(0.52, 0.52, 0.52)
    }
    pub fn select() -> Color {
        Color::rgb(0.24, 0.37, 0.58)
    }
    pub fn tab_active() -> Color {
        Color::rgb(0.28, 0.28, 0.28)
    }
    pub fn play_green() -> Color {
        Color::rgb(0.23, 0.50, 0.30)
    }
    pub fn splitter() -> Color {
        Color::rgb(0.10, 0.10, 0.10)
    }
}

fn fixed_bar(h: f32, pad_x: f32) -> LayoutSpec {
    LayoutSpec {
        width: Size::Fill,
        height: Size::Px(h),
        flex_grow: 0.0,
        flex_shrink: 0.0,
        padding: Insets::symmetric(pad_x, 0.0),
        gap: 4.0,
        ..LayoutSpec::horizontal()
    }
}

fn v_body(pad: f32, gap: f32) -> LayoutSpec {
    LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, padding: Insets::all(pad), gap, ..LayoutSpec::vertical() }
}

fn dim_label(text: impl Into<String>) -> WidgetBuilder {
    label_widget().text(text).style(Style { foreground: Some(theme::text_dim()), ..Style::default() })
}

fn bright_label(text: impl Into<String>) -> WidgetBuilder {
    label_widget().text(text).style(Style { foreground: Some(theme::text()), ..Style::default() })
}

fn menu_item(label: &str, cmd: u64) -> WidgetBuilder {
    button_widget().text(label).on_click(UiCommand::Custom(cmd)).style(Style {
        foreground: Some(theme::text()),
        corner_radius: Some(2.0),
        ..Style::default()
    })
}

fn tool_toggle(label: &str, cmd: u64, active: bool) -> WidgetBuilder {
    let bg = if active { Some(theme::select()) } else { None };
    let fg = if active { Color::rgb(0.95, 0.97, 1.0) } else { theme::text() };
    button_widget().text(label).on_click(UiCommand::Custom(cmd)).style(Style {
        background: bg,
        foreground: Some(fg),
        corner_radius: Some(3.0),
        ..Style::default()
    })
}

fn tab_btn(label: &str, cmd: u64, active: bool) -> WidgetBuilder {
    let bg = if active { Some(theme::tab_active()) } else { None };
    let fg = if active { theme::text() } else { theme::text_dim() };
    button_widget().text(label).on_click(UiCommand::Custom(cmd)).style(Style {
        background: bg,
        foreground: Some(fg),
        corner_radius: Some(0.0),
        ..Style::default()
    })
}

fn play_ctrl(label: &str, cmd: u64, lit: bool) -> WidgetBuilder {
    let bg = if lit { Some(theme::play_green()) } else { None };
    let fg = if lit { Color::rgb(0.95, 1.0, 0.95) } else { theme::text() };
    button_widget().text(label).on_click(UiCommand::Custom(cmd)).style(Style {
        background: bg,
        foreground: Some(fg),
        corner_radius: Some(3.0),
        ..Style::default()
    })
}

fn v_splitter() -> WidgetBuilder {
    panel()
        .layout(LayoutSpec { width: Size::Px(1.0), height: Size::Fill, flex_grow: 0.0, flex_shrink: 0.0, ..LayoutSpec::vertical() })
        .style(Style { background: Some(theme::splitter()), ..Style::default() })
}

fn h_splitter() -> WidgetBuilder {
    panel()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Px(1.0), flex_grow: 0.0, flex_shrink: 0.0, ..LayoutSpec::horizontal() })
        .style(Style { background: Some(theme::splitter()), ..Style::default() })
}

/// 侧栏停靠窗（Hierarchy / Inspector）：固定宽、吃满中栏高度。
fn side_dock(title: &str, width: f32, body: impl IntoIterator<Item = WidgetBuilder>) -> WidgetBuilder {
    let header = row()
        .layout(LayoutSpec {
            width: Size::Fill,
            height: Size::Px(HEADER_H),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            padding: Insets::symmetric(8.0, 0.0),
            gap: 6.0,
            ..LayoutSpec::horizontal()
        })
        .style(Style { background: Some(theme::panel_header()), ..Style::default() })
        .child(bright_label(title));

    let mut content = column().layout(v_body(6.0, 2.0)).style(Style { background: Some(theme::panel()), ..Style::default() });
    for c in body {
        content = content.child(c);
    }

    panel()
        .layout(LayoutSpec { width: Size::Px(width), height: Size::Fill, flex_grow: 0.0, flex_shrink: 0.0, gap: 0.0, ..LayoutSpec::vertical() })
        .style(Style { background: Some(theme::splitter()), ..Style::default() })
        .child(header)
        .child(content)
}

fn menu_bar(project: &ProjectInfo) -> WidgetBuilder {
    let short_root = project.root.file_name().and_then(|s| s.to_str()).unwrap_or(".");
    row()
        .layout(fixed_bar(MENU_H, 8.0))
        .style(Style { background: Some(theme::menu()), ..Style::default() })
        .child(bright_label("Sparkle"))
        .child(menu_item("File", CMD_FILE_SAVE))
        .child(menu_item("Edit", CMD_EDIT_UNDO))
        .child(menu_item("Assets", CMD_FILE_SAVE))
        .child(menu_item("GameObject", CMD_FILE_SAVE))
        .child(menu_item("Component", CMD_EDIT_UNDO))
        .child(menu_item("Window", CMD_WINDOW_GALLERY))
        .child(menu_item("Help", CMD_FILE_SAVE))
        .child(spacer_widget())
        .child(dim_label(format!("{} — {} ({})", project.name, project.kind.label(), short_root)))
}

/// 左：QWER 工具；中：Play 控件；右：Layout。
fn toolbar(state: &EditorState) -> WidgetBuilder {
    let playing = state.play != PlayMode::Edit;

    let left = row()
        .layout(LayoutSpec { height: Size::Fill, flex_grow: 1.0, gap: 4.0, justify: Justify::Start, ..LayoutSpec::horizontal() })
        .child(tool_toggle("Q Hand", CMD_TOOL_HAND, state.tool == Tool::Hand))
        .child(tool_toggle("W Move", CMD_TOOL_MOVE, state.tool == Tool::Move))
        .child(tool_toggle("E Rotate", CMD_TOOL_ROTATE, state.tool == Tool::Rotate))
        .child(tool_toggle("R Scale", CMD_TOOL_SCALE, state.tool == Tool::Scale))
        .child(dim_label("|"))
        .child(dim_label("2D"));

    let center = row()
        .layout(LayoutSpec {
            height: Size::Fill,
            flex_grow: 0.0,
            flex_shrink: 0.0,
            gap: 6.0,
            justify: Justify::Center,
            ..LayoutSpec::horizontal()
        })
        .child(play_ctrl("▶", CMD_PLAY, !playing && state.play == PlayMode::Edit))
        .child(play_ctrl("❚❚", CMD_PAUSE, state.play == PlayMode::Paused))
        .child(play_ctrl("■", CMD_STOP, playing))
        .child(play_ctrl("Step", CMD_STEP, false));

    let right = row()
        .layout(LayoutSpec { height: Size::Fill, flex_grow: 1.0, gap: 8.0, justify: Justify::End, ..LayoutSpec::horizontal() })
        .child(dim_label("Layers: Default"))
        .child(dim_label("Layout: Default"));

    // 两侧等权 spacer 行，中间 Play 视觉居中（Unity Toolbar）。
    row()
        .layout(fixed_bar(TOOLBAR_H, 8.0))
        .style(Style { background: Some(theme::toolbar()), ..Style::default() })
        .child(left)
        .child(center)
        .child(right)
}

fn hierarchy_panel(project: &ProjectInfo, state: &EditorState) -> WidgetBuilder {
    let mut body = Vec::new();
    for row in hierarchy_for(project.kind) {
        let indent = "  ".repeat(row.depth as usize);
        let mark = if row.id == state.selected { "● " } else { "○ " };
        let label = format!("{mark}{indent}{}", row.name);
        let active = row.id == state.selected;
        let mut btn = button_widget().text(label).on_click(UiCommand::Custom(select_cmd(row.id)));
        btn = btn.style(Style {
            background: if active { Some(theme::select()) } else { None },
            foreground: Some(if active { Color::rgb(0.95, 0.97, 1.0) } else { theme::text() }),
            corner_radius: Some(2.0),
            ..Style::default()
        });
        body.push(btn);
    }
    side_dock("Hierarchy", HIERARCHY_W, body)
}

fn inspector_panel(project: &ProjectInfo, state: &EditorState) -> WidgetBuilder {
    let mut body = Vec::new();
    if let Some(e) = entity_by_id(project.kind, state.selected) {
        body.push(bright_label(e.name));
        body.push(dim_label("Tag: Untagged    Layer: Default"));
        body.push(dim_label(format!("✓ {}", e.component_summary)));
        body.push(dim_label(""));
        body.push(bright_label("Transform"));
        body.push(dim_label("  Position   X 0   Y 0   Z 0"));
        body.push(dim_label("  Rotation   X 0   Y 0   Z 0"));
        body.push(dim_label("  Scale      X 1   Y 1   Z 1"));
        body.push(dim_label(""));
        let hint = match project.kind {
            ProjectKind::Rust => "组件字段来自 Rust 类型注册",
            ProjectKind::Valkyrie => "组件字段来自 Valkyrie（*.script）",
            ProjectKind::Hybrid => "Rust 与 Valkyrie 组件可挂载",
        };
        body.push(dim_label(hint));
        body.push(dim_label(""));
        body.push(menu_item("Add Component", CMD_EDIT_UNDO));
    }
    else {
        body.push(dim_label("No selection"));
        body.push(dim_label("Select an object in Hierarchy"));
    }
    side_dock("Inspector", INSPECTOR_W, body)
}

fn center_viewport(project: &ProjectInfo, state: &EditorState) -> WidgetBuilder {
    let mode = match state.play {
        PlayMode::Edit => "Edit",
        PlayMode::Play => "Play",
        PlayMode::Paused => "Paused",
    };
    let scene = project.startup_scene.as_deref().unwrap_or("(no startupScene)");
    let selected = entity_by_id(project.kind, state.selected).map(|e| e.name).unwrap_or("None");

    let tab_strip = row()
        .layout(LayoutSpec {
            width: Size::Fill,
            height: Size::Px(TAB_H),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            padding: Insets::symmetric(4.0, 0.0),
            gap: 2.0,
            ..LayoutSpec::horizontal()
        })
        .style(Style { background: Some(theme::panel_header()), ..Style::default() })
        .child(tab_btn("Scene", CMD_TAB_SCENE, state.center == CenterTab::Scene))
        .child(tab_btn("Game", CMD_TAB_GAME, state.center == CenterTab::Game))
        .child(tab_btn("Script", CMD_TAB_SCRIPT, state.center == CenterTab::Script))
        .child(spacer_widget())
        .child(dim_label(mode));

    let mut view_children = vec![
        bright_label(match state.center {
            CenterTab::Scene => "Scene",
            CenterTab::Game => "Game",
            CenterTab::Script => "Script",
        }),
        dim_label(format!("Scene asset: {scene}")),
        dim_label(format!("Selected: {selected}")),
        dim_label(""),
    ];

    match state.center {
        CenterTab::Scene => {
            view_children.push(dim_label(match project.kind {
                ProjectKind::Rust => "2D Scene — entities from native registration",
                ProjectKind::Valkyrie => "2D Scene — entities from Sparkle Script",
                ProjectKind::Hybrid => "2D Scene — Rust register then Sparkle Script",
            }));
            view_children.push(dim_label("Gizmo / grid (viewport placeholder)"));
        }
        CenterTab::Game => {
            if state.play == PlayMode::Edit {
                view_children.push(dim_label("Press ▶ in the toolbar to enter Play Mode"));
            }
            else {
                view_children.push(dim_label("Game view is live (drawn over this panel)"));
                view_children.push(dim_label("■ Stop or Esc returns to Edit · Scene tab keeps the editor chrome"));
            }
        }
        CenterTab::Script => match project.kind {
            ProjectKind::Rust => {
                view_children.push(dim_label(project.cargo_manifest.as_deref().unwrap_or("Cargo.toml / src/")));
                view_children.push(dim_label("Open sources in external IDE (embed later)"));
            }
            ProjectKind::Valkyrie => {
                view_children.push(dim_label(project.script_entry.as_deref().unwrap_or("assets/scripts/")));
                view_children.push(dim_label("Double-click *.script in Project"));
            }
            ProjectKind::Hybrid => {
                view_children.push(dim_label("Rust src/ and assets/scripts/"));
            }
        },
    }

    let viewport =
        column().layout(v_body(12.0, 6.0)).style(Style { background: Some(theme::viewport()), ..Style::default() }).children(view_children);

    panel()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, flex_shrink: 1.0, gap: 0.0, ..LayoutSpec::vertical() })
        .style(Style { background: Some(theme::splitter()), ..Style::default() })
        .child(tab_strip)
        .child(viewport)
}

/// 仅中栏底部：Project / Console / Problems（Unity Project 窗）。
fn bottom_dock(asset_lines: &[String], state: &EditorState) -> WidgetBuilder {
    let tabs = row()
        .layout(LayoutSpec {
            width: Size::Fill,
            height: Size::Px(TAB_H),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            padding: Insets::symmetric(4.0, 0.0),
            gap: 2.0,
            ..LayoutSpec::horizontal()
        })
        .style(Style { background: Some(theme::panel_header()), ..Style::default() })
        .child(tab_btn("Project", CMD_BOTTOM_PROJECT, state.bottom == BottomTab::Project))
        .child(tab_btn("Console", CMD_BOTTOM_CONSOLE, state.bottom == BottomTab::Console))
        .child(tab_btn("Problems", CMD_BOTTOM_PROBLEMS, state.bottom == BottomTab::Problems));

    let mut body_kids = Vec::new();
    match state.bottom {
        BottomTab::Project => {
            body_kids.push(dim_label("Assets"));
            if asset_lines.is_empty() {
                body_kids.push(dim_label("  (empty — create assets/ and set spark.startupScene)"));
            }
            else {
                for line in asset_lines.iter().take(24) {
                    body_kids.push(dim_label(format!("  {line}")));
                }
            }
        }
        BottomTab::Console => {
            body_kids.push(dim_label(if state.status.is_empty() { "Ready".into() } else { state.status.clone() }));
            body_kids.push(dim_label("Clear · Collapse · Error Pause (placeholder)"));
        }
        BottomTab::Problems => {
            body_kids.push(dim_label("0 Errors  ·  0 Warnings  ·  0 Messages"));
        }
    }

    let body = column().layout(v_body(8.0, 2.0)).style(Style { background: Some(theme::panel()), ..Style::default() }).children(body_kids);

    panel()
        .layout(LayoutSpec {
            width: Size::Fill,
            height: Size::Px(BOTTOM_H),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            gap: 0.0,
            ..LayoutSpec::vertical()
        })
        .style(Style { background: Some(theme::splitter()), ..Style::default() })
        .child(tabs)
        .child(body)
}

fn status_bar(state: &EditorState) -> WidgetBuilder {
    let mode = match state.play {
        PlayMode::Edit => "Edit Mode",
        PlayMode::Play => "Play Mode",
        PlayMode::Paused => "Paused",
    };
    row()
        .layout(fixed_bar(STATUS_H, 8.0))
        .style(Style { background: Some(theme::status()), ..Style::default() })
        .child(dim_label(mode))
        .child(dim_label("|"))
        .child(dim_label(if state.status.is_empty() { "Ready".into() } else { state.status.clone() }))
}

/// Unity Default：左右全高，中栏上下为 Scene + Project。
pub fn build_shell(project: &ProjectInfo, state: &EditorState, asset_lines: &[String]) -> WidgetBuilder {
    let center_column = column()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, flex_shrink: 1.0, gap: 0.0, ..LayoutSpec::vertical() })
        .child(center_viewport(project, state))
        .child(h_splitter())
        .child(bottom_dock(asset_lines, state));

    let main = row()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, flex_shrink: 1.0, gap: 0.0, ..LayoutSpec::horizontal() })
        .child(hierarchy_panel(project, state))
        .child(v_splitter())
        .child(center_column)
        .child(v_splitter())
        .child(inspector_panel(project, state));

    column()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, gap: 0.0, ..LayoutSpec::vertical() })
        .style(Style { background: Some(theme::bg()), ..Style::default() })
        .child(menu_bar(project))
        .child(toolbar(state))
        .child(h_splitter())
        .child(main)
        .child(status_bar(state))
}
