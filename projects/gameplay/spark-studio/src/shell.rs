//! 默认工作区 Widget 树（flex 近似停靠；正式 Dock 尚未进引擎）。

use spark_core::Color;
use spark_widget::{
    button_widget, column, label_widget, panel, row, spacer_widget, Insets, LayoutSpec, Size,
    Style, UiCommand, WidgetBuilder,
};

/// 自定义命令：Play 占位。
pub const CMD_PLAY: u64 = 1;
/// 自定义命令：打开 Widget Gallery 占位提示。
pub const CMD_GALLERY: u64 = 2;

fn fill() -> LayoutSpec {
    LayoutSpec {
        width: Size::Fill,
        height: Size::Fill,
        flex_grow: 1.0,
        ..LayoutSpec::vertical()
    }
}

fn bar_h(h: f32) -> LayoutSpec {
    LayoutSpec {
        width: Size::Fill,
        height: Size::Px(h),
        padding: Insets::symmetric(8.0, 4.0),
        gap: 8.0,
        ..LayoutSpec::horizontal()
    }
}

fn side_panel(title: &str, width: f32, bg: Color) -> WidgetBuilder {
    panel()
        .layout(LayoutSpec {
            width: Size::Px(width),
            height: Size::Fill,
            padding: Insets::all(8.0),
            gap: 6.0,
            ..LayoutSpec::vertical()
        })
        .style(Style {
            background: Some(bg),
            corner_radius: Some(0.0),
            ..Style::default()
        })
        .child(
            label_widget()
                .text(title)
                .style(Style {
                    foreground: Some(Color::rgb(0.75, 0.82, 0.90)),
                    ..Style::default()
                }),
        )
        .child(
            label_widget()
                .text("（骨架面板）")
                .style(Style {
                    foreground: Some(Color::rgb(0.55, 0.60, 0.66)),
                    ..Style::default()
                }),
        )
}

fn center_panel(title: &str, bg: Color) -> WidgetBuilder {
    panel()
        .layout(LayoutSpec {
            width: Size::Fill,
            height: Size::Fill,
            flex_grow: 1.0,
            padding: Insets::all(12.0),
            gap: 8.0,
            ..LayoutSpec::vertical()
        })
        .style(Style {
            background: Some(bg),
            corner_radius: Some(0.0),
            ..Style::default()
        })
        .child(
            label_widget()
                .text(title)
                .style(Style {
                    foreground: Some(Color::rgb(0.90, 0.93, 0.96)),
                    ..Style::default()
                }),
        )
        .child(
            label_widget()
                .text("用公开 spark-widget API 构建 · Esc 退出")
                .style(Style {
                    foreground: Some(Color::rgb(0.55, 0.60, 0.66)),
                    ..Style::default()
                }),
        )
}

/// 构建 Studio 默认壳布局。
pub fn build_shell(project_label: &str) -> WidgetBuilder {
    let menu = row()
        .layout(bar_h(36.0))
        .style(Style {
            background: Some(Color::rgb(0.10, 0.11, 0.14)),
            ..Style::default()
        })
        .child(button_widget().text("File").on_click(UiCommand::Custom(10)))
        .child(button_widget().text("Edit").on_click(UiCommand::Custom(11)))
        .child(button_widget().text("View").on_click(UiCommand::Custom(12)))
        .child(
            button_widget()
                .text("Play")
                .on_click(UiCommand::Custom(CMD_PLAY)),
        )
        .child(
            button_widget()
                .text("Widget Gallery")
                .on_click(UiCommand::Custom(CMD_GALLERY)),
        )
        .child(spacer_widget())
        .child(
            label_widget()
                .text(format!("项目: {project_label}"))
                .style(Style {
                    foreground: Some(Color::rgb(0.70, 0.76, 0.82)),
                    ..Style::default()
                }),
        );

    let toolbar = row()
        .layout(bar_h(32.0))
        .style(Style {
            background: Some(Color::rgb(0.12, 0.13, 0.16)),
            ..Style::default()
        })
        .child(
            label_widget()
                .text("Spark Studio · Widget dogfood shell")
                .style(Style {
                    foreground: Some(Color::rgb(0.60, 0.68, 0.78)),
                    ..Style::default()
                }),
        );

    let mid = row()
        .layout(fill())
        .child(side_panel("Hierarchy", 220.0, Color::rgb(0.11, 0.12, 0.15)))
        .child(center_panel("Scene / Game", Color::rgb(0.07, 0.08, 0.10)))
        .child(side_panel("Inspector", 280.0, Color::rgb(0.11, 0.12, 0.15)));

    let bottom = row()
        .layout(LayoutSpec {
            width: Size::Fill,
            height: Size::Px(160.0),
            ..LayoutSpec::horizontal()
        })
        .child(side_panel("Project", 220.0, Color::rgb(0.10, 0.11, 0.14)))
        .child(center_panel("Console / Problems", Color::rgb(0.09, 0.10, 0.12)));

    column()
        .layout(LayoutSpec {
            width: Size::Fill,
            height: Size::Fill,
            ..LayoutSpec::vertical()
        })
        .style(Style {
            background: Some(Color::rgb(0.08, 0.09, 0.11)),
            ..Style::default()
        })
        .child(menu)
        .child(toolbar)
        .child(mid)
        .child(bottom)
}
