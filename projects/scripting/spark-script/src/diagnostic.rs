//! 编译诊断批次（统一模型入口）。

use spark_diagnostics::{Diagnostic, DiagnosticLabel, DiagnosticNote, ErrorArgs, ErrorCode, MessageKey, Severity, SourceSpan};

use crate::request::LanguageProfileId;

/// 附带语言 profile 的诊断（渲染前仍走 `spark-diagnostics`）。
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptDiagnostic {
    /// 底层统一诊断模型（码、严重级、标签、笔记）。
    pub diagnostic: Diagnostic,
    /// 产生本诊断的语言 profile（若已知）。
    pub language_profile: Option<LanguageProfileId>,
    /// 源文件逻辑名或路径（若已知）。
    pub source_file: Option<std::sync::Arc<str>>,
}

impl ScriptDiagnostic {
    /// 构造错误级诊断；可选主跨度写入标签。
    pub fn error(code: &str, args: ErrorArgs, primary: Option<SourceSpan>) -> Self {
        let code = ErrorCode::parse(code);
        let mut diag = Diagnostic::error(code).with_args(args);
        if let Some(span) = primary {
            diag.labels.push(DiagnosticLabel { span, message_key: None });
        }
        Self { diagnostic: diag, language_profile: None, source_file: None }
    }

    /// 附上语言 profile 元数据。
    pub fn with_profile(mut self, profile: LanguageProfileId) -> Self {
        self.language_profile = Some(profile);
        self
    }

    /// 附上源文件名 / 路径元数据。
    pub fn with_source_file(mut self, file: impl Into<std::sync::Arc<str>>) -> Self {
        self.source_file = Some(file.into());
        self
    }

    /// 追加一条笔记（消息键 + 参数）。
    pub fn note(mut self, message_key: MessageKey, args: ErrorArgs) -> Self {
        self.diagnostic.notes.push(DiagnosticNote { message_key, args });
        self
    }

    /// 当前严重级别。
    pub fn severity(&self) -> Severity {
        self.diagnostic.severity
    }
}

/// 一次编译会话收集的诊断。
#[derive(Debug, Clone, Default)]
pub struct DiagnosticBatch {
    /// 按出现顺序累计的诊断项。
    pub items: Vec<ScriptDiagnostic>,
}

impl DiagnosticBatch {
    /// 空批次。
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一条诊断。
    pub fn push(&mut self, item: ScriptDiagnostic) {
        self.items.push(item);
    }

    /// 是否含 `Error` 或 `Bug` 级诊断。
    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| matches!(d.severity(), Severity::Error | Severity::Bug))
    }

    /// `Error` / `Bug` 级诊断数量。
    pub fn error_count(&self) -> usize {
        self.items.iter().filter(|d| matches!(d.severity(), Severity::Error | Severity::Bug)).count()
    }
}
