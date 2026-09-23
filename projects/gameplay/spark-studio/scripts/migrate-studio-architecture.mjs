#!/usr/bin/env node
/**
 * 强制将 spark-studio 迁到 ui/ + layout/ + state/ 目标结构。
 * 不留兼容层：删除顶层 shell.rs / state.rs，重写 lib.rs。
 */
import { mkdir, writeFile, unlink, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SRC = path.resolve(__dirname, "../src");

/** @type {Record<string, string>} */
const FILES = {
  "lib.rs": `//! Spark Studio 库面（供 \`tests/\` 与 bin 共用）。
//!
//! 编辑器壳在 \`ui/\`，停靠布局在 \`layout/\`，会话状态在 \`state/\`。

#![forbid(missing_docs)]

pub mod app;
pub mod layout;
pub mod play;
pub mod project;
pub mod state;
pub mod ui;
`,

  "layout/mod.rs": `//! 编辑器停靠布局：尺寸、分栏与持久化。

mod dock;
mod persistence;
mod preset;

pub use dock::*;
pub use persistence::*;
pub use preset::*;
`,

  "layout/preset.rs": `//! 布局预设：默认 / 脚本 / 调试。

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
}
`,

  "layout/persistence.rs": `//! 停靠布局状态（后续按项目持久化）。

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
`,

  "ui/mod.rs": `//! 编辑器 Widget 壳：顶栏、面板与编排入口。

mod asset_browser;
mod console;
mod hierarchy;
mod inspector;
mod problems;
mod scene_toolbar;
mod shell;
mod status_bar;
mod style;
mod top_bar;
mod viewport;

pub use shell::build_shell;
`,

  "ui/style.rs": `//! 编辑器壳共享主题与控件构造辅助。

use spark_types::Color;
use spark_widget::{Insets, LayoutSpec, Size, Style, Theme, UiCommand, WidgetBuilder, button_widget, label_widget};

/// 编辑器石墨色主题。
pub(crate) fn editor_theme() -> Theme {
    Theme::editor_dark()
}

/// 主文字色。
pub(crate) fn text_primary() -> Color {
    editor_theme().colors.foreground
}

/// 次要文字色（仍保持可读对比度）。
pub(crate) fn text_secondary() -> Color {
    editor_theme().colors.foreground_secondary
}

/// 活动选中表面色。
pub(crate) fn selected_surface() -> Color {
    editor_theme().colors.selection
}

/// 面板表面色。
pub(crate) fn panel_surface() -> Color {
    editor_theme().colors.surface
}

/// 应用 chrome 背景色。
pub(crate) fn chrome_surface() -> Color {
    editor_theme().colors.background
}

/// 分隔线色。
pub(crate) fn divider() -> Color {
    editor_theme().colors.border
}

/// 固定高度横条布局。
pub(crate) fn fixed_bar(h: f32, pad_x: f32) -> LayoutSpec {
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

/// 面板正文纵向布局。
pub(crate) fn v_body(pad: f32, gap: f32) -> LayoutSpec {
    LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, padding: Insets::all(pad), gap, ..LayoutSpec::vertical() }
}

/// 弱提示标签。
pub(crate) fn dim_label(text: impl Into<String>) -> WidgetBuilder {
    label_widget().text(text).style(Style { foreground: Some(text_secondary()), ..Style::default() })
}

/// 主文字标签。
pub(crate) fn bright_label(text: impl Into<String>) -> WidgetBuilder {
    label_widget().text(text).style(Style { foreground: Some(text_primary()), ..Style::default() })
}

/// 透明底菜单项。
pub(crate) fn menu_item(label: &str, cmd: u64) -> WidgetBuilder {
    button_widget().text(label).on_click(UiCommand::Custom(cmd)).style(Style {
        background: Some(Color::rgba(0.0, 0.0, 0.0, 0.0)),
        foreground: Some(text_primary()),
        corner_radius: Some(0.0),
        ..Style::default()
    })
}

/// 工具栏互斥切换按钮。
pub(crate) fn tool_toggle(label: &str, cmd: u64, active: bool) -> WidgetBuilder {
    let bg = if active { Some(selected_surface()) } else { Some(Color::rgba(0.0, 0.0, 0.0, 0.0)) };
    button_widget().text(label).on_click(UiCommand::Custom(cmd)).style(Style {
        background: bg,
        foreground: Some(text_primary()),
        corner_radius: Some(2.0),
        ..Style::default()
    })
}

/// 页签按钮。
pub(crate) fn tab_btn(label: &str, cmd: u64, active: bool) -> WidgetBuilder {
    let bg = if active { Some(panel_surface()) } else { Some(Color::rgba(0.0, 0.0, 0.0, 0.0)) };
    let fg = if active { text_primary() } else { text_secondary() };
    button_widget().text(label).on_click(UiCommand::Custom(cmd)).style(Style {
        background: bg,
        foreground: Some(fg),
        corner_radius: Some(0.0),
        ..Style::default()
    })
}

/// Play 控件按钮。
pub(crate) fn play_ctrl(label: &str, cmd: u64, lit: bool, enabled: bool) -> WidgetBuilder {
    let bg = if lit { Some(selected_surface()) } else { Some(Color::rgba(0.0, 0.0, 0.0, 0.0)) };
    button_widget()
        .text(label)
        .on_click(UiCommand::Custom(cmd))
        .style(Style { background: bg, foreground: Some(text_primary()), corner_radius: Some(2.0), ..Style::default() })
        .disabled(!enabled)
}
`,

  "ui/top_bar.rs": `//! 全局主栏：项目名与文件/编辑/窗口入口。

use spark_widget::{Style, WidgetBuilder, row, spacer_widget};

use crate::{
    layout::DockLayoutState,
    project::ProjectInfo,
    state::{CMD_EDIT_UNDO, CMD_FILE_SAVE, CMD_WINDOW_GALLERY},
    ui::style::{bright_label, chrome_surface, dim_label, fixed_bar, menu_item},
};

/// 构建顶栏菜单行。
pub fn build_top_bar(project: &ProjectInfo) -> WidgetBuilder {
    let short_root = project.root.file_name().and_then(|s| s.to_str()).unwrap_or(".");
    row()
        .layout(fixed_bar(DockLayoutState::MENU_HEIGHT, 8.0))
        .style(Style { background: Some(chrome_surface()), ..Style::default() })
        .child(bright_label("Spark Studio"))
        .child(menu_item("文件", CMD_FILE_SAVE))
        .child(menu_item("编辑", CMD_EDIT_UNDO))
        .child(menu_item("窗口", CMD_WINDOW_GALLERY))
        .child(spacer_widget())
        .child(dim_label(format!("{} — {} ({})", project.name, project.kind.label(), short_root)))
}
`,

  "ui/scene_toolbar.rs": `//! Scene 上下文工具栏与 Play 控件。

use spark_widget::{Insets, Justify, LayoutSpec, Size, Style, WidgetBuilder, row, spacer_widget};

use crate::{
    layout::DockLayoutState,
    state::{CMD_PAUSE, CMD_PLAY, CMD_STOP, CMD_TOOL_HAND, CMD_TOOL_MOVE, CMD_TOOL_ROTATE, CMD_TOOL_SCALE, EditorState, PlayMode, Tool},
    ui::style::{dim_label, fixed_bar, panel_surface, play_ctrl, tool_toggle},
};

/// 构建场景工具栏。
pub fn build_scene_toolbar(state: &EditorState) -> WidgetBuilder {
    let playing = state.play != PlayMode::Edit;

    let left = row()
        .layout(LayoutSpec { height: Size::Fill, flex_grow: 1.0, gap: 4.0, justify: Justify::Start, ..LayoutSpec::horizontal() })
        .child(tool_toggle("选择", CMD_TOOL_HAND, state.tool == Tool::Hand))
        .child(tool_toggle("移动", CMD_TOOL_MOVE, state.tool == Tool::Move))
        .child(tool_toggle("旋转", CMD_TOOL_ROTATE, state.tool == Tool::Rotate))
        .child(tool_toggle("缩放", CMD_TOOL_SCALE, state.tool == Tool::Scale))
        .child(dim_label("2D 场景"));

    let center = row()
        .layout(LayoutSpec {
            height: Size::Fill,
            flex_grow: 0.0,
            flex_shrink: 0.0,
            gap: 6.0,
            justify: Justify::Center,
            ..LayoutSpec::horizontal()
        })
        .child(play_ctrl("运行", CMD_PLAY, state.play == PlayMode::Play, true))
        .child(play_ctrl("暂停", CMD_PAUSE, state.play == PlayMode::Paused, playing))
        .child(play_ctrl("停止", CMD_STOP, playing, playing));

    let right = row()
        .layout(LayoutSpec { height: Size::Fill, flex_grow: 1.0, gap: 8.0, justify: Justify::End, ..LayoutSpec::horizontal() })
        .child(dim_label("默认图层"))
        .child(dim_label("默认布局"));

    row()
        .layout(fixed_bar(DockLayoutState::TOOLBAR_HEIGHT, 8.0))
        .style(Style { background: Some(panel_surface()), ..Style::default() })
        .child(left)
        .child(center)
        .child(right)
}
`,

  "ui/hierarchy.rs": `//! 层级面板。

use spark_types::Color;
use spark_widget::{Style, UiCommand, WidgetBuilder, button_widget};

use crate::{
    layout::{DockLayoutState, side_dock},
    project::ProjectInfo,
    state::{EditorState, hierarchy_for, select_cmd},
    ui::style::{selected_surface, text_primary},
};

/// 构建层级树面板。
pub fn build_hierarchy_panel(project: &ProjectInfo, state: &EditorState, dock: &DockLayoutState) -> WidgetBuilder {
    let mut body = Vec::new();
    for row in hierarchy_for(project.kind) {
        let indent = "  ".repeat(row.depth as usize);
        let label = format!("{indent}{}", row.name);
        let active = row.id == state.selected;
        let btn = button_widget().text(label).on_click(UiCommand::Custom(select_cmd(row.id))).style(Style {
            background: if active { Some(selected_surface()) } else { Some(Color::rgba(0.0, 0.0, 0.0, 0.0)) },
            foreground: Some(text_primary()),
            corner_radius: Some(0.0),
            ..Style::default()
        });
        body.push(btn);
    }
    side_dock("层级", dock.hierarchy_width, body)
}
`,

  "ui/inspector.rs": `//! 检查器面板。

use spark_widget::WidgetBuilder;

use crate::{
    layout::{DockLayoutState, side_dock},
    project::{ProjectInfo, ProjectKind},
    state::{CMD_EDIT_UNDO, EditorState, entity_by_id},
    ui::style::{bright_label, dim_label, menu_item},
};

/// 构建检查器面板。
pub fn build_inspector_panel(project: &ProjectInfo, state: &EditorState, dock: &DockLayoutState) -> WidgetBuilder {
    let mut body = Vec::new();
    if let Some(e) = entity_by_id(project.kind, state.selected) {
        body.push(bright_label(e.name));
        body.push(dim_label("标签：未设置    图层：默认"));
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
        body.push(menu_item("添加组件", CMD_EDIT_UNDO).disabled(true));
    }
    else {
        body.push(dim_label("未选择对象"));
        body.push(dim_label("请在层级面板中选择对象"));
    }
    side_dock("检查器", dock.inspector_width, body)
}
`,

  "ui/viewport.rs": `//! 中央 Scene / Game / Script 视口。

use spark_widget::{Insets, LayoutSpec, Size, Style, WidgetBuilder, column, panel, row, spacer_widget};

use crate::{
    layout::DockLayoutState,
    project::{ProjectInfo, ProjectKind},
    state::{CMD_TAB_GAME, CMD_TAB_SCENE, CMD_TAB_SCRIPT, CenterTab, EditorState, PlayMode, entity_by_id},
    ui::style::{bright_label, chrome_surface, dim_label, divider, tab_btn, v_body},
};

/// 构建中央视口（含页签条与正文）。
pub fn build_viewport(project: &ProjectInfo, state: &EditorState) -> WidgetBuilder {
    let mode = match state.play {
        PlayMode::Edit => "编辑模式",
        PlayMode::Play => "运行中",
        PlayMode::Paused => "已暂停",
    };
    let scene = project.startup_scene.as_deref().unwrap_or("未设置启动场景");
    let selected = entity_by_id(project.kind, state.selected).map(|e| e.name).unwrap_or("未选择");

    let tab_strip = row()
        .layout(LayoutSpec {
            width: Size::Fill,
            height: Size::Px(DockLayoutState::TAB_HEIGHT),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            padding: Insets::symmetric(4.0, 0.0),
            gap: 2.0,
            ..LayoutSpec::horizontal()
        })
        .style(Style { background: Some(chrome_surface()), ..Style::default() })
        .child(tab_btn("场景", CMD_TAB_SCENE, state.center == CenterTab::Scene))
        .child(tab_btn("游戏", CMD_TAB_GAME, state.center == CenterTab::Game))
        .child(tab_btn("脚本", CMD_TAB_SCRIPT, state.center == CenterTab::Script))
        .child(spacer_widget())
        .child(dim_label(mode));

    let mut view_children = vec![
        bright_label(match state.center {
            CenterTab::Scene => "场景",
            CenterTab::Game => "游戏",
            CenterTab::Script => "脚本",
        }),
        dim_label(format!("场景资源：{scene}")),
        dim_label(format!("当前选择：{selected}")),
        dim_label(""),
    ];

    match state.center {
        CenterTab::Scene => {
            view_children.push(dim_label(match project.kind {
                ProjectKind::Rust => "2D 场景 · 原生注册实体",
                ProjectKind::Valkyrie => "2D 场景 · Valkyrie 脚本实体",
                ProjectKind::Hybrid => "2D 场景 · Rust 与 Valkyrie 混合实体",
            }));
            view_children.push(dim_label("场景画布尚未接入网格与变换工具"));
        }
        CenterTab::Game => {
            if state.play == PlayMode::Edit {
                view_children.push(dim_label("点击工具栏中的“运行”进入游戏模式"));
            }
            else {
                view_children.push(dim_label("游戏视图正在运行"));
                view_children.push(dim_label("点击“停止”或按 Esc 返回编辑模式"));
            }
        }
        CenterTab::Script => match project.kind {
            ProjectKind::Rust => {
                view_children.push(dim_label(project.cargo_manifest.as_deref().unwrap_or("Cargo.toml / src/")));
                view_children.push(dim_label("在外部编辑器中打开源码"));
            }
            ProjectKind::Valkyrie => {
                view_children.push(dim_label(project.script_entry.as_deref().unwrap_or("assets/scripts/")));
                view_children.push(dim_label("在项目面板中双击 *.script 文件"));
            }
            ProjectKind::Hybrid => {
                view_children.push(dim_label("Rust src/ and assets/scripts/"));
            }
        },
    }

    let viewport =
        column().layout(v_body(12.0, 6.0)).style(Style { background: Some(chrome_surface()), ..Style::default() }).children(view_children);

    panel()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, flex_shrink: 1.0, gap: 0.0, ..LayoutSpec::vertical() })
        .style(Style { background: Some(divider()), ..Style::default() })
        .child(tab_strip)
        .child(viewport)
}
`,

  "ui/asset_browser.rs": `//! 项目资源面板正文。

use spark_widget::WidgetBuilder;

use crate::ui::style::dim_label;

/// 构建项目资源列表正文。
pub fn build_asset_browser_body(asset_lines: &[String]) -> Vec<WidgetBuilder> {
    let mut body = vec![dim_label("资源")];
    if asset_lines.is_empty() {
        body.push(dim_label("  暂无资源，请创建 assets/ 并设置 spark.startupScene"));
    }
    else {
        for line in asset_lines.iter().take(24) {
            body.push(dim_label(format!("  {line}")));
        }
    }
    body
}
`,

  "ui/console.rs": `//! 控制台面板正文。

use spark_widget::WidgetBuilder;

use crate::ui::style::dim_label;

/// 构建控制台正文。
pub fn build_console_body(status: &str) -> Vec<WidgetBuilder> {
    vec![dim_label(if status.is_empty() { "就绪".into() } else { status.to_string() })]
}
`,

  "ui/problems.rs": `//! 问题面板正文。

use spark_widget::WidgetBuilder;

use crate::ui::style::dim_label;

/// 构建问题列表面板正文。
pub fn build_problems_body() -> Vec<WidgetBuilder> {
    vec![dim_label("0 个错误  ·  0 个警告  ·  0 条消息")]
}
`,

  "ui/status_bar.rs": `//! 底部状态栏。

use spark_widget::{Style, WidgetBuilder, row};

use crate::{
    layout::DockLayoutState,
    state::{EditorState, PlayMode},
    ui::style::{chrome_surface, dim_label, fixed_bar},
};

/// 构建状态栏。
pub fn build_status_bar(state: &EditorState) -> WidgetBuilder {
    let mode = match state.play {
        PlayMode::Edit => "编辑模式",
        PlayMode::Play => "运行模式",
        PlayMode::Paused => "已暂停",
    };
    row()
        .layout(fixed_bar(DockLayoutState::STATUS_HEIGHT, 8.0))
        .style(Style { background: Some(chrome_surface()), ..Style::default() })
        .child(dim_label(mode))
        .child(dim_label(if state.status.is_empty() { "就绪".into() } else { state.status.clone() }))
}
`,

  "layout/dock.rs": `//! 停靠分栏与底栏组装。

use spark_widget::{Insets, LayoutSpec, Size, Style, WidgetBuilder, column, panel, row};

use crate::{
    state::{BottomTab, CMD_BOTTOM_CONSOLE, CMD_BOTTOM_PROBLEMS, CMD_BOTTOM_PROJECT, EditorState},
    ui::{
        asset_browser::build_asset_browser_body,
        console::build_console_body,
        problems::build_problems_body,
        style::{chrome_surface, divider, panel_surface, tab_btn, v_body},
    },
};

use super::DockLayoutState;

/// 垂直分栏线。
pub fn v_splitter() -> WidgetBuilder {
    panel()
        .layout(LayoutSpec { width: Size::Px(1.0), height: Size::Fill, flex_grow: 0.0, flex_shrink: 0.0, ..LayoutSpec::vertical() })
        .style(Style { background: Some(divider()), ..Style::default() })
}

/// 水平分栏线。
pub fn h_splitter() -> WidgetBuilder {
    panel()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Px(1.0), flex_grow: 0.0, flex_shrink: 0.0, ..LayoutSpec::horizontal() })
        .style(Style { background: Some(divider()), ..Style::default() })
}

/// 侧栏停靠窗（层级 / 检查器）。
pub fn side_dock(title: &str, width: f32, body: impl IntoIterator<Item = WidgetBuilder>) -> WidgetBuilder {
    use crate::ui::style::{bright_label, chrome_surface, v_body};

    let header = row()
        .layout(LayoutSpec {
            width: Size::Fill,
            height: Size::Px(DockLayoutState::HEADER_HEIGHT),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            padding: Insets::symmetric(8.0, 0.0),
            gap: 6.0,
            ..LayoutSpec::horizontal()
        })
        .style(Style { background: Some(chrome_surface()), ..Style::default() })
        .child(bright_label(title));

    let mut content = column().layout(v_body(6.0, 2.0)).style(Style { background: Some(panel_surface()), ..Style::default() });
    for child in body {
        content = content.child(child);
    }

    panel()
        .layout(LayoutSpec { width: Size::Px(width), height: Size::Fill, flex_grow: 0.0, flex_shrink: 0.0, gap: 0.0, ..LayoutSpec::vertical() })
        .style(Style { background: Some(divider()), ..Style::default() })
        .child(header)
        .child(content)
}

/// 中栏底部停靠：项目 / 控制台 / 问题。
pub fn build_bottom_dock(asset_lines: &[String], state: &EditorState, dock: &DockLayoutState) -> WidgetBuilder {
    let tabs = row()
        .layout(LayoutSpec {
            width: Size::Fill,
            height: Size::Px(DockLayoutState::TAB_HEIGHT),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            padding: Insets::symmetric(4.0, 0.0),
            gap: 2.0,
            ..LayoutSpec::horizontal()
        })
        .style(Style { background: Some(chrome_surface()), ..Style::default() })
        .child(tab_btn("项目", CMD_BOTTOM_PROJECT, state.bottom == BottomTab::Project))
        .child(tab_btn("控制台", CMD_BOTTOM_CONSOLE, state.bottom == BottomTab::Console))
        .child(tab_btn("问题", CMD_BOTTOM_PROBLEMS, state.bottom == BottomTab::Problems));

    let body_kids = match state.bottom {
        BottomTab::Project => build_asset_browser_body(asset_lines),
        BottomTab::Console => build_console_body(&state.status),
        BottomTab::Problems => build_problems_body(),
    };

    let body = column().layout(v_body(8.0, 2.0)).style(Style { background: Some(panel_surface()), ..Style::default() }).children(body_kids);

    panel()
        .layout(LayoutSpec {
            width: Size::Fill,
            height: Size::Px(dock.bottom_height),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            gap: 0.0,
            ..LayoutSpec::vertical()
        })
        .style(Style { background: Some(divider()), ..Style::default() })
        .child(tabs)
        .child(body)
}
`,

  "ui/shell.rs": `//! 编辑器壳编排：组合顶栏、停靠区与状态栏。

use spark_widget::{LayoutSpec, Size, Style, WidgetBuilder, column, row};

use crate::{
    layout::{DockLayoutState, LayoutPreset, build_bottom_dock, h_splitter, v_splitter},
    project::ProjectInfo,
    state::EditorState,
    ui::{
        hierarchy::build_hierarchy_panel,
        inspector::build_inspector_panel,
        scene_toolbar::build_scene_toolbar,
        status_bar::build_status_bar,
        top_bar::build_top_bar,
        viewport::build_viewport,
        style::chrome_surface,
    },
};

/// 按当前会话状态构建完整编辑器 Widget 树。
pub fn build_shell(project: &ProjectInfo, state: &EditorState, asset_lines: &[String]) -> WidgetBuilder {
    let dock = LayoutPreset::Default.dock_layout();

    let center_column = column()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, flex_shrink: 1.0, gap: 0.0, ..LayoutSpec::vertical() })
        .child(build_viewport(project, state))
        .child(h_splitter())
        .child(build_bottom_dock(asset_lines, state, &dock));

    let main = row()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, flex_shrink: 1.0, gap: 0.0, ..LayoutSpec::horizontal() })
        .child(build_hierarchy_panel(project, state, &dock))
        .child(v_splitter())
        .child(center_column)
        .child(v_splitter())
        .child(build_inspector_panel(project, state, &dock));

    column()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, gap: 0.0, ..LayoutSpec::vertical() })
        .style(Style { background: Some(chrome_surface()), ..Style::default() })
        .child(build_top_bar(project))
        .child(build_scene_toolbar(state))
        .child(h_splitter())
        .child(main)
        .child(build_status_bar(state))
}
`,
};

const REMOVE = ["shell.rs", "state.rs"];

async function writeTree() {
  for (const [rel, content] of Object.entries(FILES)) {
    const abs = path.join(SRC, rel);
    await mkdir(path.dirname(abs), { recursive: true });
    await writeFile(abs, content.replace(/\n/g, "\r\n"), "utf8");
    console.log(`wrote ${rel}`);
  }
}

async function removeLegacy() {
  for (const rel of REMOVE) {
    const abs = path.join(SRC, rel);
    try {
      await unlink(abs);
      console.log(`removed ${rel}`);
    } catch (err) {
      if (err && typeof err === "object" && "code" in err && err.code === "ENOENT") {
        console.log(`skip missing ${rel}`);
      } else {
        throw err;
      }
    }
  }
}

await writeTree();
await removeLegacy();
console.log("spark-studio architecture migration complete");
