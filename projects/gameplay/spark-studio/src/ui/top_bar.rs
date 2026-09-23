//! 全局主栏：项目名与文件/编辑/窗口入口。

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
        .child(menu_item("编辑", CMD_EDIT_UNDO).disabled(true))
        .child(menu_item("窗口", CMD_WINDOW_GALLERY))
        .child(spacer_widget())
        .child(dim_label(format!("{} — {} ({})", project.name, project.kind.label(), short_root)))
}
