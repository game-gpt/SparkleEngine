//! 分栏分隔线。

use spark_widget::{LayoutSpec, Size, Style, WidgetBuilder, panel};

use crate::ui::style::divider;

/// 垂直分栏线。
pub fn v_splitter() -> WidgetBuilder {
    panel()
        .layout(LayoutSpec { width: Size::Px(1.0), height: Size::Fill, flex_grow: 0.0, flex_shrink: 0.0, ..LayoutSpec::vertical() })
        .style(Style { background: Some(divider()), ..Style::default() })
}

/// 水平分栏线。
pub fn h_splitter() -> WidgetBuilder {
    panel()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Px(1.0), flex_grow: 0.0, flex_shrink: 0.0, ..LayoutSpec::horizontal() })
        .style(Style { background: Some(divider()), ..Style::default() })
}
