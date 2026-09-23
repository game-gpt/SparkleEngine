//! 侧栏停靠窗 chrome。

use spark_widget::{Insets, LayoutSpec, Size, Style, WidgetBuilder, column, panel, row};

use crate::layout::DockLayoutState;
use crate::ui::style::{bright_label, chrome_surface, divider, panel_surface, v_body};

/// 侧栏停靠窗（层级 / 检查器）。
pub fn side_dock(title: &str, width: f32, body: impl IntoIterator<Item = WidgetBuilder>) -> WidgetBuilder {
    let header = row()
        .layout(LayoutSpec {
            width: Size::Fill,
            height: Size::Px(DockLayoutState::HEADER_HEIGHT),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            padding: Insets::symmetric(8.0, 0.0),
            gap: 6.0,
            ..LayoutSpec::horizontal()
        })
        .style(Style { background: Some(chrome_surface()), ..Style::default() })
        .child(bright_label(title));

    let mut content = column().layout(v_body(6.0, 2.0)).style(Style { background: Some(panel_surface()), ..Style::default() });
    for child in body {
        content = content.child(child);
    }

    panel()
        .layout(LayoutSpec { width: Size::Px(width), height: Size::Fill, flex_grow: 0.0, flex_shrink: 0.0, gap: 0.0, ..LayoutSpec::vertical() })
        .style(Style { background: Some(divider()), ..Style::default() })
        .child(header)
        .child(content)
}
