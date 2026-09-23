//! 控制台面板正文。

use spark_widget::{LayoutSpec, Size, WidgetBuilder, column, row, scroll_view, spacer_widget};

use crate::{
    state::{ConsoleLog, CMD_CONSOLE_CLEAR},
    ui::style::{bright_label, dim_label, menu_item, v_body},
};

/// 构建控制台正文（可滚动，顶栏含清空）。
pub fn build_console_body(console: &ConsoleLog) -> Vec<WidgetBuilder> {
    let header = row()
        .layout(LayoutSpec { width: Size::Fill, height: Size::Px(24.0), flex_grow: 0.0, gap: 8.0, ..LayoutSpec::horizontal() })
        .child(bright_label(format!("控制台 · {} 行", console.len())))
        .child(spacer_widget())
        .child(menu_item("清空", CMD_CONSOLE_CLEAR));

    let mut log_column = column().layout(v_body(0.0, 2.0));
    if console.len() == 0 {
        log_column = log_column.child(dim_label("  就绪"));
    }
    else {
        for line in console.lines() {
            log_column = log_column.child(dim_label(format!("  {line}")));
        }
    }

    let scroll = scroll_view()
        .key("console.scroll")
        .layout(LayoutSpec { width: Size::Fill, height: Size::Fill, flex_grow: 1.0, ..LayoutSpec::vertical() })
        .child(log_column);

    vec![header, scroll]
}
