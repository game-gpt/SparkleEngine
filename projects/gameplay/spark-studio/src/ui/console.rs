//! 控制台面板正文。

use spark_widget::WidgetBuilder;

use crate::ui::style::dim_label;

/// 构建控制台正文。
pub fn build_console_body(status: &str) -> Vec<WidgetBuilder> {
    vec![dim_label(if status.is_empty() { "就绪".into() } else { status.to_string() })]
}
