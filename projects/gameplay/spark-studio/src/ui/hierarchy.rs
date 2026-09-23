//! 层级面板。

use spark_types::Color;
use spark_widget::{Style, UiCommand, WidgetBuilder, button_widget};

use crate::{
    layout::DockLayoutState,
    ui::dock_panel::side_dock,
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
