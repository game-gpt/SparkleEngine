//! 问题面板：项目打开时的静态诊断。

use crate::project::ProjectInfo;

/// 单条诊断。
#[derive(Debug, Clone)]
pub struct Problem {
    /// 严重级别。
    pub severity: ProblemSeverity,
    /// 摘要。
    pub message: String,
}

/// 诊断严重级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemSeverity {
    /// 错误（阻塞发布或运行）。
    Error,
    /// 警告（可继续编辑）。
    Warning,
    /// 信息。
    Info,
}

impl ProblemSeverity {
    /// 列表前缀符号。
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Error => "✕",
            Self::Warning => "⚠",
            Self::Info => "ℹ",
        }
    }
}

/// 根据项目元数据与资源扫描生成诊断列表。
pub fn diagnose_project(project: &ProjectInfo, asset_count: usize) -> Vec<Problem> {
    let mut out = Vec::new();

    if project.startup_scene.is_none() {
        out.push(Problem { severity: ProblemSeverity::Warning, message: "未设置 spark.startupScene".into() });
    }
    if project.kind_inferred {
        out.push(Problem {
            severity: ProblemSeverity::Info,
            message: format!("项目 kind 为推断结果（{}），建议在 package.json 显式声明", project.kind.as_str()),
        });
    }
    if !project.has_sparkle_engine_dep {
        out.push(Problem { severity: ProblemSeverity::Warning, message: "package.json 未声明 @game-gpt/sparkle-engine 依赖".into() });
    }
    if asset_count == 0 {
        out.push(Problem { severity: ProblemSeverity::Info, message: "assets/ 下暂无可列出的资源".into() });
    }

    if out.is_empty() {
        out.push(Problem { severity: ProblemSeverity::Info, message: "未发现项目配置问题".into() });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{ProjectInfo, ProjectKind};

    #[test]
    fn diagnose_flags_missing_startup_scene() {
        let project = ProjectInfo {
            root: std::path::PathBuf::from("."),
            name: "demo".into(),
            kind: ProjectKind::Rust,
            kind_inferred: false,
            startup_scene: None,
            script_entry: None,
            cargo_manifest: None,
            run_target: None,
            has_sparkle_engine_dep: true,
        };
        let problems = diagnose_project(&project, 3);
        assert!(problems.iter().any(|p| p.message.contains("startupScene")));
    }
}
