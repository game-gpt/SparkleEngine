//! 检查器面板。

use spark_widget::{LayoutSpec, Size, WidgetBuilder, row, separator_widget, text_field_widget};

use crate::{
    layout::DockLayoutState,
    project::{ProjectInfo, ProjectKind},
    state::{CMD_EDIT_UNDO, EditorState, entity_by_id},
    ui::{
        dock_panel::side_dock,
        style::{bright_label, dim_label, menu_item},
    },
};

fn field_row(label: &str, value: &str) -> WidgetBuilder {
    row()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Px(24.0), flex_grow: 0.0, gap: 6.0, ..LayoutSpec::horizontal() })
        .child(dim_label(label))
        .child(
            text_field_widget()
                .text(value)
                .layout(LayoutSpec { width: Size::Fill, height: Size::Px(24.0), flex_grow: 1.0, ..LayoutSpec::horizontal() }),
        )
}

/// 构建检查器面板。
pub fn build_inspector_panel(project: &ProjectInfo, state: &EditorState, dock: &DockLayoutState) -> WidgetBuilder {
    let mut body = Vec::new();
    if let Some(e) = entity_by_id(project.kind, state.selected) {
        body.push(bright_label(e.name));
        body.push(dim_label("标签：未设置    图层：默认"));
        body.push(dim_label(format!("✓ {}", e.component_summary)));
        body.push(separator_widget());
        body.push(bright_label("Transform"));
        let t = &state.transform;
        body.push(field_row("Position X", &format!("{:.2}", t.pos_x)));
        body.push(field_row("Position Y", &format!("{:.2}", t.pos_y)));
        body.push(field_row("Position Z", &format!("{:.2}", t.pos_z)));
        body.push(field_row("Rotation X", &format!("{:.1}°", t.rot_x)));
        body.push(field_row("Rotation Y", &format!("{:.1}°", t.rot_y)));
        body.push(field_row("Rotation Z", &format!("{:.1}°", t.rot_z)));
        body.push(field_row("Scale X", &format!("{:.2}", t.scale_x)));
        body.push(field_row("Scale Y", &format!("{:.2}", t.scale_y)));
        body.push(field_row("Scale Z", &format!("{:.2}", t.scale_z)));
        body.push(separator_widget());
        let hint = match project.kind {
            ProjectKind::Rust => "组件字段来自 Rust 类型注册",
            ProjectKind::Valkyrie => "组件字段来自 Valkyrie（*.script）",
            ProjectKind::Hybrid => "Rust 与 Valkyrie 组件可挂载",
        };
        body.push(dim_label(hint));
        body.push(menu_item("添加组件", CMD_EDIT_UNDO).disabled(true));
    }
    else {
        body.push(dim_label("未选择对象"));
        body.push(dim_label("请在层级面板中选择对象"));
    }
    side_dock("检查器", dock.inspector_width, body)
}
