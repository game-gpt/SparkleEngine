//! 场景视口画布：网格、原点与相机框占位绘制。

use spark_types::Color;
use spark_widget::{LayoutSpec, Size, Style, WidgetBuilder, column, panel, row};

use crate::ui::style::dim_label;

/// 场景工作区深色画布表面。
fn canvas_surface() -> Color {
    Color::rgb(0.055, 0.060, 0.068)
}

/// 次网格线色。
fn grid_minor() -> Color {
    Color::rgb(0.10, 0.11, 0.13)
}

/// 主网格线色。
fn grid_major() -> Color {
    Color::rgb(0.16, 0.17, 0.20)
}

/// 构建带网格纹样的场景画布区域。
pub fn build_scene_canvas(selected_name: &str) -> WidgetBuilder {
    let mut stripes = Vec::new();
    for i in 0..18 {
        let color = if i % 6 == 0 { grid_major() } else { grid_minor() };
        stripes.push(
            panel()
                .layout(LayoutSpec { width: Size::Fill, height: Size::Px(1.0), flex_grow: 0.0, ..LayoutSpec::horizontal() })
                .style(Style { background: Some(color), ..Style::default() }),
        );
    }

    let grid = column()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, gap: 7.0, ..LayoutSpec::vertical() })
        .style(Style { background: Some(canvas_surface()), ..Style::default() })
        .children(stripes);

    let camera = panel()
        .layout(LayoutSpec { width: Size::Px(180.0), height: Size::Px(100.0), flex_grow: 0.0, flex_shrink: 0.0, ..LayoutSpec::horizontal() })
        .style(Style {
            background: Some(Color::rgba(0.0, 0.0, 0.0, 0.0)),
            border_color: Some(Color::rgb(0.35, 0.55, 0.75)),
            border_width: Some(1.0),
            ..Style::default()
        });

    column()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, gap: 6.0, ..LayoutSpec::vertical() })
        .child(dim_label(format!("世界原点 (0, 0)  ·  选中：{selected_name}")))
        .child(
            panel()
                .layout(LayoutSpec { width: Size::Fill, height: Size::Px(200.0), flex_grow: 1.0, padding: spark_widget::Insets::all(1.0), gap: 0.0, ..LayoutSpec::vertical() })
                .style(Style { background: Some(grid_major()), border_color: Some(grid_major()), border_width: Some(1.0), ..Style::default() })
                .child(grid),
        )
        .child(row().layout(LayoutSpec { width: Size::Fill, height: Size::Px(100.0), justify: spark_widget::Justify::Center, align: spark_widget::Align::Center, ..LayoutSpec::horizontal() }).child(camera))
        .child(dim_label("相机预览框  ·  滚轮缩放与平移将在后续迭代接入"))
}
