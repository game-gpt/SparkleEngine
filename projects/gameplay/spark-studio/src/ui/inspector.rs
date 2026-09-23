//! 检查器面板。

use spark_widget::WidgetBuilder;

use crate::{
    layout::DockLayoutState,
    ui::dock_panel::side_dock,
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
