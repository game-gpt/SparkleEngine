//! 项目资源面板正文。

use spark_types::Color;
use spark_widget::{Style, UiCommand, WidgetBuilder, button_widget};

use crate::{
    state::{EditorState, asset_cmd},
    ui::style::{dim_label, selected_surface, text_primary},
};

/// 构建项目资源列表正文。
pub fn build_asset_browser_body(asset_lines: &[String], state: &EditorState) -> Vec<WidgetBuilder> {
    let mut body = vec![dim_label("资源")];
    if asset_lines.is_empty() {
        body.push(dim_label("  暂无资源，请创建 assets/ 并设置 spark.startupScene"));
    }
    else {
        for (i, line) in asset_lines.iter().take(24).enumerate() {
            let active = state.selected_asset == Some(i as u32);
            let label = format!("  {line}");
            body.push(
                button_widget()
                    .key(format!("asset.row.{i}"))
                    .text(label)
                    .on_click(UiCommand::Custom(asset_cmd(i as u64)))
                    .style(Style {
                        background: if active { Some(selected_surface()) } else { Some(Color::rgba(0.0, 0.0, 0.0, 0.0)) },
                        foreground: Some(text_primary()),
                        corner_radius: Some(0.0),
                        ..Style::default()
                    }),
            );
        }
    }
    body
}
