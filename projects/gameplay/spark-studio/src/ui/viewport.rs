//! 中央 Scene / Game / Script 视口。

use spark_types::Color;
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

    let scene_overlay = state.center == CenterTab::Scene;
    let body_bg = if scene_overlay { Color::rgba(0.0, 0.0, 0.0, 0.0) } else { chrome_surface() };

    let mut view_children: Vec<WidgetBuilder> = Vec::new();

    match state.center {
        CenterTab::Scene => {
            view_children.push(dim_label(format!("场景资源：{scene}")));
            view_children.push(dim_label(format!("当前选择：{selected}")));
            view_children.push(spacer_widget());
        }
        CenterTab::Game => {
            view_children.push(bright_label("游戏"));
            view_children.push(dim_label(format!("场景资源：{scene}")));
            if state.play == PlayMode::Edit {
                view_children.push(dim_label("点击工具栏中的“运行”进入游戏模式"));
            }
            else {
                view_children.push(dim_label("游戏视图正在运行"));
                view_children.push(dim_label("点击“停止”或按 Esc 返回编辑模式"));
            }
        }
        CenterTab::Script => {
            view_children.push(bright_label("脚本"));
            match project.kind {
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
            }
        }
    }

    let viewport = column()
        .layout(v_body(8.0, 4.0))
        .style(Style { background: Some(body_bg), ..Style::default() })
        .children(view_children);

    panel()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, flex_shrink: 1.0, gap: 0.0, ..LayoutSpec::vertical() })
        .style(Style { background: Some(divider()), ..Style::default() })
        .child(tab_strip)
        .child(viewport)
}
