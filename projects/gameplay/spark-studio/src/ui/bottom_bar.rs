//! 中栏底部停靠：项目 / 控制台 / 问题。

use spark_widget::{Insets, LayoutSpec, Size, Style, WidgetBuilder, column, panel, row};

use crate::{
    layout::DockLayoutState,
    state::{BottomTab, CMD_BOTTOM_CONSOLE, CMD_BOTTOM_PROBLEMS, CMD_BOTTOM_PROJECT, EditorState},
    ui::{
        asset_browser::build_asset_browser_body,
        console::build_console_body,
        problems::build_problems_body,
        style::{chrome_surface, divider, panel_surface, tab_btn, v_body},
    },
};

/// 构建底栏停靠区。
pub fn build_bottom_bar(asset_lines: &[String], state: &EditorState, dock: &DockLayoutState) -> WidgetBuilder {
    let tabs = row()
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
        .child(tab_btn("项目", CMD_BOTTOM_PROJECT, state.bottom == BottomTab::Project))
        .child(tab_btn("控制台", CMD_BOTTOM_CONSOLE, state.bottom == BottomTab::Console))
        .child(tab_btn("问题", CMD_BOTTOM_PROBLEMS, state.bottom == BottomTab::Problems));

    let body_kids = match state.bottom {
        BottomTab::Project => build_asset_browser_body(asset_lines, state),
        BottomTab::Console => build_console_body(&state.status),
        BottomTab::Problems => build_problems_body(),
    };

    let body = column().layout(v_body(8.0, 2.0)).style(Style { background: Some(panel_surface()), ..Style::default() }).children(body_kids);

    panel()
        .layout(LayoutSpec {
            width: Size::Fill,
            height: Size::Px(dock.bottom_height),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            gap: 0.0,
            ..LayoutSpec::vertical()
        })
        .style(Style { background: Some(divider()), ..Style::default() })
        .child(tabs)
        .child(body)
}
