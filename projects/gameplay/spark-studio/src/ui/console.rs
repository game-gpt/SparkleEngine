//! 控制台面板正文。

use spark_widget::WidgetBuilder;

use crate::{state::ConsoleLog, ui::style::dim_label};

/// 构建控制台正文（最近日志在底部）。
pub fn build_console_body(console: &ConsoleLog) -> Vec<WidgetBuilder> {
    let mut body = Vec::new();
    let lines: Vec<_> = console.lines().collect();
    if lines.is_empty() {
        body.push(dim_label("  就绪"));
        return body;
    }
    for line in lines.iter().rev().take(32) {
        body.push(dim_label(format!("  {line}")));
    }
    body
}
