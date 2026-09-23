//! 问题面板正文。

use spark_widget::WidgetBuilder;

use crate::{
    project::ProjectInfo,
    state::{ProblemSeverity, diagnose_project},
    ui::style::dim_label,
};

/// 构建问题列表面板正文。
pub fn build_problems_body(project: &ProjectInfo, asset_count: usize) -> Vec<WidgetBuilder> {
    let problems = diagnose_project(project, asset_count);
    let errors = problems.iter().filter(|p| p.severity == ProblemSeverity::Error).count();
    let warnings = problems.iter().filter(|p| p.severity == ProblemSeverity::Warning).count();
    let infos = problems.len() - errors - warnings;

    let mut body = vec![dim_label(format!("{errors} 个错误  ·  {warnings} 个警告  ·  {infos} 条消息"))];
    for p in problems {
        body.push(dim_label(format!("  {} {}", p.severity.glyph(), p.message)));
    }
    body
}
