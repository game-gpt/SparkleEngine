//! 底部状态栏。

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
