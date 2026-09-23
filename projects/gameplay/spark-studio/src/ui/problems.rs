//! 问题面板正文。

use spark_widget::WidgetBuilder;

use crate::ui::style::dim_label;

/// 构建问题列表面板正文。
pub fn build_problems_body() -> Vec<WidgetBuilder> {
    vec![dim_label("0 个错误  ·  0 个警告  ·  0 条消息")]
}
