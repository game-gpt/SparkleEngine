//! Scene 上下文工具栏与 Play 控件。

use spark_widget::{Justify, LayoutSpec, Size, Style, WidgetBuilder, row};

use crate::{
    layout::DockLayoutState,
    state::{
        CMD_LAYOUT_DEBUG, CMD_LAYOUT_DEFAULT, CMD_LAYOUT_SCRIPT, CMD_PAUSE, CMD_PLAY, CMD_STEP, CMD_STOP, CMD_TOOL_HAND, CMD_TOOL_MOVE,
        CMD_TOOL_ROTATE, CMD_TOOL_SCALE, EditorState, PlayMode, Tool,
    },
    ui::style::{dim_label, fixed_bar, panel_surface, play_ctrl, tool_toggle},
};

/// 构建场景工具栏。
pub fn build_scene_toolbar(state: &EditorState) -> WidgetBuilder {
    let playing = state.play != PlayMode::Edit;
    let paused = state.play == PlayMode::Paused;

    let play_enabled = state.play == PlayMode::Edit || state.play == PlayMode::Paused;
    let play_lit = state.play == PlayMode::Play;

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
        .child(play_ctrl("运行", CMD_PLAY, play_lit, play_enabled))
        .child(play_ctrl("暂停", CMD_PAUSE, paused, playing))
        .child(play_ctrl("停止", CMD_STOP, playing, playing))
        .child(play_ctrl("单步", CMD_STEP, false, paused));

    let right = row()
        .layout(LayoutSpec { height: Size::Fill, flex_grow: 1.0, gap: 6.0, justify: Justify::End, ..LayoutSpec::horizontal() })
        .child(dim_label("默认图层"))
        .child(tool_toggle("默认", CMD_LAYOUT_DEFAULT, state.layout_preset == crate::layout::LayoutPreset::Default))
        .child(tool_toggle("脚本", CMD_LAYOUT_SCRIPT, state.layout_preset == crate::layout::LayoutPreset::Script))
        .child(tool_toggle("调试", CMD_LAYOUT_DEBUG, state.layout_preset == crate::layout::LayoutPreset::Debug));

    row()
        .layout(fixed_bar(DockLayoutState::TOOLBAR_HEIGHT, 8.0))
        .style(Style { background: Some(panel_surface()), ..Style::default() })
        .child(left)
        .child(center)
        .child(right)
}
