//! Scene 上下文工具栏与 Play 控件。

use spark_widget::{Justify, LayoutSpec, Size, Style, WidgetBuilder, row};

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
