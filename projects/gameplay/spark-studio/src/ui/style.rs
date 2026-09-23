//! 编辑器壳共享主题与控件构造辅助。

use spark_types::Color;
use spark_widget::{Insets, LayoutSpec, Size, Style, Theme, UiCommand, WidgetBuilder, button_widget, label_widget};

/// 编辑器石墨色主题。
pub(crate) fn editor_theme() -> Theme {
    Theme::editor_dark()
}

/// 主文字色。
pub(crate) fn text_primary() -> Color {
    editor_theme().colors.foreground
}

/// 次要文字色（仍保持可读对比度）。
pub(crate) fn text_secondary() -> Color {
    editor_theme().colors.foreground_secondary
}

/// 活动选中表面色。
pub(crate) fn selected_surface() -> Color {
    editor_theme().colors.selection
}

/// 面板表面色。
pub(crate) fn panel_surface() -> Color {
    editor_theme().colors.surface
}

/// 应用 chrome 背景色。
pub(crate) fn chrome_surface() -> Color {
    editor_theme().colors.background
}

/// 分隔线色。
pub(crate) fn divider() -> Color {
    editor_theme().colors.border
}

/// 固定高度横条布局。
pub(crate) fn fixed_bar(h: f32, pad_x: f32) -> LayoutSpec {
    LayoutSpec {
        width: Size::Fill,
        height: Size::Px(h),
        flex_grow: 0.0,
        flex_shrink: 0.0,
        padding: Insets::symmetric(pad_x, 0.0),
        gap: 4.0,
        ..LayoutSpec::horizontal()
    }
}

/// 面板正文纵向布局。
pub(crate) fn v_body(pad: f32, gap: f32) -> LayoutSpec {
    LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, padding: Insets::all(pad), gap, ..LayoutSpec::vertical() }
}

/// 弱提示标签。
pub(crate) fn dim_label(text: impl Into<String>) -> WidgetBuilder {
    label_widget().text(text).style(Style { foreground: Some(text_secondary()), ..Style::default() })
}

/// 主文字标签。
pub(crate) fn bright_label(text: impl Into<String>) -> WidgetBuilder {
    label_widget().text(text).style(Style { foreground: Some(text_primary()), ..Style::default() })
}

/// 透明底菜单项。
pub(crate) fn menu_item(label: &str, cmd: u64) -> WidgetBuilder {
    button_widget().text(label).on_click(UiCommand::Custom(cmd)).style(Style {
        background: Some(Color::rgba(0.0, 0.0, 0.0, 0.0)),
        foreground: Some(text_primary()),
        corner_radius: Some(0.0),
        ..Style::default()
    })
}

/// 工具栏互斥切换按钮。
pub(crate) fn tool_toggle(label: &str, cmd: u64, active: bool) -> WidgetBuilder {
    let bg = if active { Some(selected_surface()) } else { Some(Color::rgba(0.0, 0.0, 0.0, 0.0)) };
    button_widget().text(label).on_click(UiCommand::Custom(cmd)).style(Style {
        background: bg,
        foreground: Some(text_primary()),
        corner_radius: Some(2.0),
        ..Style::default()
    })
}

/// 页签按钮。
pub(crate) fn tab_btn(label: &str, cmd: u64, active: bool) -> WidgetBuilder {
    let bg = if active { Some(panel_surface()) } else { Some(Color::rgba(0.0, 0.0, 0.0, 0.0)) };
    let fg = if active { text_primary() } else { text_secondary() };
    button_widget().text(label).on_click(UiCommand::Custom(cmd)).style(Style {
        background: bg,
        foreground: Some(fg),
        corner_radius: Some(0.0),
        ..Style::default()
    })
}

/// Play 控件按钮。
pub(crate) fn play_ctrl(label: &str, cmd: u64, lit: bool, enabled: bool) -> WidgetBuilder {
    let bg = if lit { Some(selected_surface()) } else { Some(Color::rgba(0.0, 0.0, 0.0, 0.0)) };
    button_widget()
        .text(label)
        .on_click(UiCommand::Custom(cmd))
        .style(Style { background: bg, foreground: Some(text_primary()), corner_radius: Some(2.0), ..Style::default() })
        .disabled(!enabled)
}
