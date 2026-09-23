//! 底部状态栏。

use spark_types::Color;
use spark_widget::{Style, WidgetBuilder, row, spacer_widget};

use crate::{
    layout::DockLayoutState,
    project::ProjectInfo,
    state::{CenterTab, EditorState, PlayMode, entity_by_id},
    ui::style::{chrome_surface, dim_label, fixed_bar, text_primary},
};

/// 构建状态栏。
pub fn build_status_bar(project: &ProjectInfo, state: &EditorState) -> WidgetBuilder {
    let (mode, bar_bg) = match state.play {
        PlayMode::Edit => ("编辑模式", chrome_surface()),
        PlayMode::Play => ("运行中", Color::rgb(0.12, 0.22, 0.16)),
        PlayMode::Paused => ("已暂停", Color::rgb(0.20, 0.18, 0.10)),
    };
    let mode_color = if state.play == PlayMode::Edit { text_primary() } else { Color::rgb(0.75, 0.90, 0.78) };

    row()
        .layout(fixed_bar(DockLayoutState::STATUS_HEIGHT, 8.0))
        .style(Style { background: Some(bar_bg), ..Style::default() })
        .child(
            dim_label(mode)
                .style(Style { foreground: Some(mode_color), ..Style::default() }),
        )
        .child(dim_label(if state.status.is_empty() { "就绪".into() } else { state.status.clone() }))
        .child(spacer_widget())
        .child(dim_label(scene_hint(project, state)))
        .child(dim_label("F 聚焦 · Ctrl+↑↓ UI · Ctrl+J 底栏"))
}

fn scene_hint(project: &ProjectInfo, state: &EditorState) -> String {
    if state.center != CenterTab::Scene {
        return String::new();
    }
    let zoom = (state.viewport.zoom * 100.0).round();
    let name = entity_by_id(project.kind, state.selected).map(|e| e.name).unwrap_or("—");
    let ui_pct = (state.ui_scale * 100.0).round();
    format!("场景 {zoom}% · {name} · UI {ui_pct}%")
}
