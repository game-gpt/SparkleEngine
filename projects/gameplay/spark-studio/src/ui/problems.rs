//! 问题面板正文。

use spark_widget::WidgetBuilder;

use crate::{
    project::ProjectInfo,
    state::{Problem, collect_problems, count_by_severity},
    ui::style::dim_label,
};

/// 构建问题列表面板正文。
pub fn build_problems_body(project: &ProjectInfo, asset_count: usize, runtime: &[Problem]) -> Vec<WidgetBuilder> {
    let problems = collect_problems(project, asset_count, runtime);
    let (errors, warnings, infos) = count_by_severity(&problems);

    let mut body = vec![dim_label(format!("{errors} 个错误  ·  {warnings} 个警告  ·  {infos} 条消息"))];
    for p in problems {
        body.push(dim_label(format!("  {} {}", p.severity.glyph(), p.message)));
    }
    body
}
