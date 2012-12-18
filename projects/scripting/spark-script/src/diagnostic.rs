//! 编译诊断批次（统一模型入口）。

use spark_diagnostics::{Diagnostic, DiagnosticLabel, DiagnosticNote, ErrorArgs, ErrorCode, MessageKey, Severity, SourceSpan};

use crate::request::LanguageProfileId;

/// 附带语言 profile 的诊断（渲染前仍走 `spark-diagnostics`）。
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptDiagnostic {
    pub diagnostic: Diagnostic,
    pub language_profile: Option<LanguageProfileId>,
    pub source_file: Option<std::sync::Arc<str>>,
}

impl ScriptDiagnostic {
    pub fn error(code: &str, args: ErrorArgs, primary: Option<SourceSpan>) -> Self {
        let code = ErrorCode::parse(code);
        let mut diag = Diagnostic::error(code).with_args(args);
        if let Some(span) = primary {
            diag.labels.push(DiagnosticLabel { span, message_key: None });
        }
        Self { diagnostic: diag, language_profile: None, source_file: None }
    }

    pub fn with_profile(mut self, profile: LanguageProfileId) -> Self {
        self.language_profile = Some(profile);
        self
    }

    pub fn with_source_file(mut self, file: impl Into<std::sync::Arc<str>>) -> Self {
        self.source_file = Some(file.into());
        self
    }

    pub fn note(mut self, message_key: MessageKey, args: ErrorArgs) -> Self {
        self.diagnostic.notes.push(DiagnosticNote { message_key, args });
        self
    }

    pub fn severity(&self) -> Severity {
        self.diagnostic.severity
    }
}

/// 一次编译会话收集的诊断。
#[derive(Debug, Clone, Default)]
pub struct DiagnosticBatch {
    pub items: Vec<ScriptDiagnostic>,
}

impl DiagnosticBatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, item: ScriptDiagnostic) {
        self.items.push(item);
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| matches!(d.severity(), Severity::Error | Severity::Bug))
    }

    pub fn error_count(&self) -> usize {
        self.items.iter().filter(|d| matches!(d.severity(), Severity::Error | Severity::Bug)).count()
    }
}
