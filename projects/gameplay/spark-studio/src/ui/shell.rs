//! 编辑器壳编排：组合顶栏、停靠区与状态栏。

use spark_widget::{LayoutSpec, Size, Style, WidgetBuilder, column, row};

use crate::{
    project::ProjectInfo,
    state::EditorState,
    ui::{
        bottom_bar::build_bottom_bar,
        hierarchy::build_hierarchy_panel,
        inspector::build_inspector_panel,
        scene_toolbar::build_scene_toolbar,
        splitter::{h_splitter, v_splitter},
        status_bar::build_status_bar,
        style::chrome_surface,
        top_bar::build_top_bar,
        viewport::build_viewport,
    },
};

/// 按当前会话状态构建完整编辑器 Widget 树。
pub fn build_shell(project: &ProjectInfo, state: &EditorState, asset_lines: &[String]) -> WidgetBuilder {
    let dock = state.dock;

    let mut center_column = column()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, flex_shrink: 1.0, gap: 0.0, ..LayoutSpec::vertical() })
        .child(build_viewport(project, state));
    if !dock.bottom_collapsed {
        center_column = center_column.child(h_splitter()).child(build_bottom_bar(project, asset_lines, state, &dock));
    }

    let mut main = row().layout(LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, flex_shrink: 1.0, gap: 0.0, ..LayoutSpec::horizontal() });
    if !dock.hierarchy_collapsed {
        main = main.child(build_hierarchy_panel(project, state, &dock)).child(v_splitter());
    }
    main = main.child(center_column);
    if !dock.inspector_collapsed {
        main = main.child(v_splitter()).child(build_inspector_panel(project, state, &dock));
    }

    column()
        .key("studio.shell")
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, gap: 0.0, ..LayoutSpec::vertical() })
        .style(Style { background: Some(chrome_surface()), ..Style::default() })
        .child(build_top_bar(project))
        .child(build_scene_toolbar(state))
        .child(h_splitter())
        .child(main)
        .child(build_status_bar(project, state))
}
