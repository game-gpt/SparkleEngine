//! Unity-like 默认工作区（flex 近似停靠）。

use spark_types::Color;
use spark_widget::{Insets, LayoutSpec, Size, Style, UiCommand, WidgetBuilder, button_widget, column, label_widget, panel, row, spacer_widget};

use crate::{
    project::{ProjectInfo, ProjectKind},
    state::{
        BottomTab, CMD_BOTTOM_CONSOLE, CMD_BOTTOM_PROBLEMS, CMD_BOTTOM_PROJECT, CMD_EDIT_UNDO, CMD_FILE_SAVE, CMD_PAUSE, CMD_PLAY, CMD_STEP,
        CMD_STOP, CMD_TAB_GAME, CMD_TAB_SCENE, CMD_TAB_SCRIPT, CMD_TOOL_HAND, CMD_TOOL_MOVE, CMD_TOOL_ROTATE, CMD_TOOL_SCALE,
        CMD_WINDOW_GALLERY, CenterTab, EditorState, PlayMode, Tool, entity_by_id, hierarchy_for, select_cmd,
    },
};

fn fill() -> LayoutSpec {
    LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, ..LayoutSpec::vertical() }
}

fn bar_h(h: f32) -> LayoutSpec {
    LayoutSpec { width: Size::Fill, height: Size::Px(h), padding: Insets::symmetric(8.0, 4.0), gap: 6.0, ..LayoutSpec::horizontal() }
}

fn menu_btn(label: &str, cmd: u64) -> WidgetBuilder {
    button_widget().text(label).on_click(UiCommand::Custom(cmd))
}

fn tool_btn(label: &str, cmd: u64, active: bool) -> WidgetBuilder {
    let mut b = button_widget().text(label).on_click(UiCommand::Custom(cmd));
    if active {
        b = b.style(Style {
            background: Some(Color::rgb(0.22, 0.45, 0.72)),
            foreground: Some(Color::rgb(0.95, 0.97, 1.0)),
            corner_radius: Some(3.0),
            ..Style::default()
        });
    }
    b
}

fn play_btn(label: &str, cmd: u64, accent: bool) -> WidgetBuilder {
    let mut b = button_widget().text(label).on_click(UiCommand::Custom(cmd));
    if accent {
        b = b.style(Style {
            background: Some(Color::rgb(0.18, 0.55, 0.28)),
            foreground: Some(Color::rgb(0.95, 1.0, 0.95)),
            corner_radius: Some(3.0),
            ..Style::default()
        });
    }
    b
}

fn dim_label(text: impl Into<String>) -> WidgetBuilder {
    label_widget().text(text).style(Style { foreground: Some(Color::rgb(0.55, 0.60, 0.66)), ..Style::default() })
}

fn bright_label(text: impl Into<String>) -> WidgetBuilder {
    label_widget().text(text).style(Style { foreground: Some(Color::rgb(0.90, 0.93, 0.96)), ..Style::default() })
}

fn panel_chrome(title: &str, width: Option<f32>, bg: Color, children: impl IntoIterator<Item = WidgetBuilder>) -> WidgetBuilder {
    let mut layout = LayoutSpec { height: Size::Fill, padding: Insets::all(8.0), gap: 4.0, ..LayoutSpec::vertical() };
    if let Some(w) = width {
        layout.width = Size::Px(w);
    }
    else {
        layout.width = Size::Fill;
        layout.flex_grow = 1.0;
    }
    let mut p = panel()
        .layout(layout)
        .style(Style { background: Some(bg), corner_radius: Some(0.0), ..Style::default() })
        .child(label_widget().text(title).style(Style { foreground: Some(Color::rgb(0.72, 0.78, 0.86)), ..Style::default() }));
    for c in children {
        p = p.child(c);
    }
    p
}

fn hierarchy_panel(project: &ProjectInfo, state: &EditorState) -> WidgetBuilder {
    let mut children = Vec::new();
    for row in hierarchy_for(project.kind) {
        let indent = "  ".repeat(row.depth as usize);
        let mark = if row.id == state.selected { "▸ " } else { "  " };
        let label = format!("{mark}{indent}{}", row.name);
        let mut btn = button_widget().text(label).on_click(UiCommand::Custom(select_cmd(row.id)));
        if row.id == state.selected {
            btn = btn.style(Style {
                background: Some(Color::rgb(0.20, 0.38, 0.62)),
                foreground: Some(Color::rgb(0.95, 0.97, 1.0)),
                corner_radius: Some(2.0),
                ..Style::default()
            });
        }
        children.push(btn);
    }
    panel_chrome("Hierarchy", Some(240.0), Color::rgb(0.14, 0.15, 0.17), children)
}

fn inspector_panel(project: &ProjectInfo, state: &EditorState) -> WidgetBuilder {
    let mut children = Vec::new();
    if let Some(e) = entity_by_id(project.kind, state.selected) {
        children.push(bright_label(e.name));
        children.push(dim_label(format!("Enabled ✓ · {}", e.component_summary)));
        children.push(dim_label("—"));
        children.push(bright_label("Transform"));
        children.push(dim_label("  Position   0.0 , 0.0 , 0.0"));
        children.push(dim_label("  Rotation   0.0"));
        children.push(dim_label("  Scale      1.0 , 1.0"));
        match project.kind {
            ProjectKind::Rust => {
                children.push(dim_label("—"));
                children.push(dim_label("字段来自 Rust 类型注册（元数据接通后可编辑）"));
            }
            ProjectKind::Valkyrie => {
                children.push(dim_label("—"));
                children.push(dim_label("字段来自 *.script（f32 / bool / string）"));
            }
            ProjectKind::Hybrid => {
                children.push(dim_label("—"));
                children.push(dim_label("Rust 与 Valkyrie 组件可同时挂载"));
            }
        }
        children.push(dim_label("—"));
        children.push(menu_btn("Add Component", CMD_EDIT_UNDO));
    }
    else {
        children.push(dim_label("未选择对象"));
        children.push(dim_label("在 Hierarchy 中点选实体"));
    }
    panel_chrome("Inspector", Some(300.0), Color::rgb(0.14, 0.15, 0.17), children)
}

fn scene_panel(project: &ProjectInfo, state: &EditorState) -> WidgetBuilder {
    let mode = match state.play {
        PlayMode::Edit => "Edit",
        PlayMode::Play => "Play",
        PlayMode::Paused => "Paused",
    };
    let tab = match state.center {
        CenterTab::Scene => "Scene",
        CenterTab::Game => "Game",
        CenterTab::Script => "Script",
    };
    let scene = project.startup_scene.as_deref().unwrap_or("(未配置 startupScene)");
    let selected = entity_by_id(project.kind, state.selected).map(|e| e.name).unwrap_or("—");

    let mut children = vec![
        row()
            .layout(LayoutSpec { width: Size::Fill, height: Size::Px(28.0), gap: 6.0, ..LayoutSpec::horizontal() })
            .child(tool_btn("Scene", CMD_TAB_SCENE, state.center == CenterTab::Scene))
            .child(tool_btn("Game", CMD_TAB_GAME, state.center == CenterTab::Game))
            .child(tool_btn("Script", CMD_TAB_SCRIPT, state.center == CenterTab::Script)),
        bright_label(format!("{tab} · {mode} · {}", project.kind.label())),
        dim_label(format!("场景  {scene}")),
        dim_label(format!("选中  {selected}")),
        dim_label("—"),
    ];

    match state.center {
        CenterTab::Scene => {
            children.push(bright_label("2D Scene"));
            children.push(dim_label(match project.kind {
                ProjectKind::Rust => "纯 Rust：实体权威来自 native 注册",
                ProjectKind::Valkyrie => "纯 Valkyrie：实体权威来自 *.script",
                ProjectKind::Hybrid => "混合：先 Rust 注册，再加载脚本",
            }));
        }
        CenterTab::Game => {
            children.push(bright_label("Game View"));
            if state.play == PlayMode::Edit {
                children.push(dim_label("按 Play 嵌入运行当前示例对局"));
            }
            else {
                let hint = match project.kind {
                    ProjectKind::Rust => format!(
                        "Play 中：{}（也可 cargo run -p {}）",
                        project.name,
                        project.run_target.as_deref().unwrap_or(project.name.as_str())
                    ),
                    ProjectKind::Valkyrie => "Play 中：试玩宿主（VM 接通前）".into(),
                    ProjectKind::Hybrid => format!("Play 中：{}（脚本元数据仍可检视）", project.run_target.as_deref().unwrap_or("native")),
                };
                children.push(dim_label(hint));
            }
        }
        CenterTab::Script => {
            children.push(bright_label("Script"));
            match project.kind {
                ProjectKind::Rust => {
                    children.push(dim_label(project.cargo_manifest.as_deref().unwrap_or("Cargo.toml / src/")));
                    children.push(dim_label("打开 Rust 源码（外部 IDE / 后续内嵌）"));
                }
                ProjectKind::Valkyrie => {
                    children.push(dim_label(project.script_entry.as_deref().unwrap_or("assets/scripts/")));
                    children.push(dim_label("双击 Project 中的 *.script"));
                }
                ProjectKind::Hybrid => {
                    children.push(dim_label("Rust src/ 与 assets/scripts/ 均可打开"));
                }
            }
        }
    }

    panel_chrome(" ", None, Color::rgb(0.08, 0.09, 0.10), children)
}

fn project_panel(asset_lines: &[String], state: &EditorState) -> WidgetBuilder {
    let mut children = vec![
        row()
            .layout(LayoutSpec { width: Size::Fill, height: Size::Px(28.0), gap: 6.0, ..LayoutSpec::horizontal() })
            .child(tool_btn("Project", CMD_BOTTOM_PROJECT, state.bottom == BottomTab::Project))
            .child(tool_btn("Console", CMD_BOTTOM_CONSOLE, state.bottom == BottomTab::Console))
            .child(tool_btn("Problems", CMD_BOTTOM_PROBLEMS, state.bottom == BottomTab::Problems)),
    ];

    match state.bottom {
        BottomTab::Project => {
            if asset_lines.is_empty() {
                children.push(dim_label("未找到 assets/。可创建 assets/scenes 并设置 spark.startupScene。"));
            }
            else {
                for line in asset_lines.iter().take(24) {
                    children.push(dim_label(line.clone()));
                }
            }
        }
        BottomTab::Console => {
            children.push(dim_label(if state.status.is_empty() { "就绪".into() } else { state.status.clone() }));
            children.push(dim_label("日志将显示在此"));
        }
        BottomTab::Problems => {
            children.push(dim_label("0 errors · 0 warnings"));
        }
    }

    panel()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Px(180.0), padding: Insets::all(8.0), gap: 4.0, ..LayoutSpec::vertical() })
        .style(Style { background: Some(Color::rgb(0.12, 0.13, 0.15)), corner_radius: Some(0.0), ..Style::default() })
        .children(children)
}

/// 构建完整编辑器壳。
pub fn build_shell(project: &ProjectInfo, state: &EditorState, asset_lines: &[String]) -> WidgetBuilder {
    let menu = row()
        .layout(bar_h(32.0))
        .style(Style { background: Some(Color::rgb(0.11, 0.12, 0.14)), ..Style::default() })
        .child(bright_label("Spark"))
        .child(menu_btn("File", CMD_FILE_SAVE))
        .child(menu_btn("Edit", CMD_EDIT_UNDO))
        .child(menu_btn("Assets", CMD_FILE_SAVE))
        .child(menu_btn("GameObject", CMD_FILE_SAVE))
        .child(menu_btn("Component", CMD_EDIT_UNDO))
        .child(menu_btn("Window", CMD_WINDOW_GALLERY))
        .child(menu_btn("Help", CMD_FILE_SAVE))
        .child(spacer_widget())
        .child(dim_label(format!("{} · {} ({}) — {}", project.name, project.kind.label(), project.kind.as_str(), project.root.display())));

    let toolbar = row()
        .layout(bar_h(36.0))
        .style(Style { background: Some(Color::rgb(0.13, 0.14, 0.16)), ..Style::default() })
        .child(tool_btn("Hand", CMD_TOOL_HAND, state.tool == Tool::Hand))
        .child(tool_btn("Move", CMD_TOOL_MOVE, state.tool == Tool::Move))
        .child(tool_btn("Rotate", CMD_TOOL_ROTATE, state.tool == Tool::Rotate))
        .child(tool_btn("Scale", CMD_TOOL_SCALE, state.tool == Tool::Scale))
        .child(dim_label("│"))
        .child(dim_label("2D"))
        .child(dim_label("Center"))
        .child(spacer_widget())
        .child(play_btn("▶ Play", CMD_PLAY, state.play == PlayMode::Edit))
        .child(play_btn("❚❚", CMD_PAUSE, state.play == PlayMode::Paused))
        .child(menu_btn("Step", CMD_STEP))
        .child(menu_btn("Stop", CMD_STOP));

    let mid =
        row().layout(fill()).child(hierarchy_panel(project, state)).child(scene_panel(project, state)).child(inspector_panel(project, state));

    column()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, ..LayoutSpec::vertical() })
        .style(Style { background: Some(Color::rgb(0.10, 0.11, 0.12)), ..Style::default() })
        .child(menu)
        .child(toolbar)
        .child(mid)
        .child(project_panel(asset_lines, state))
}
