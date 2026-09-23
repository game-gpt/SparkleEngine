//! 项目资源面板正文。

use spark_widget::WidgetBuilder;

use crate::ui::style::dim_label;

/// 构建项目资源列表正文。
pub fn build_asset_browser_body(asset_lines: &[String]) -> Vec<WidgetBuilder> {
    let mut body = vec![dim_label("资源")];
    if asset_lines.is_empty() {
        body.push(dim_label("  暂无资源，请创建 assets/ 并设置 spark.startupScene"));
    }
    else {
        for line in asset_lines.iter().take(24) {
            body.push(dim_label(format!("  {line}")));
        }
    }
    body
}
