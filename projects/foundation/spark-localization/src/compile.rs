//! 将作者文档编译为 [`LocalizationBundle`]。

use crate::{
    bundle::{CompiledMessage, LocalizationBundle},
    check::{CheckReport, check_document},
    document::LocalizationDocument,
};

/// 编译错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// 文档检查未通过。
    CheckFailed { count: usize },
    /// 编译阶段事实不足，`detail` 仅供调试器展开。
    Internal { detail: String },
}

impl CompileError {
    pub fn check_failed(count: usize) -> Self {
        Self::CheckFailed { count }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::CheckFailed { .. } => "spark.localization.check_failed",
            Self::Internal { .. } => "spark.localization.compile_internal",
        }
    }

    pub fn args(&self) -> spark_core::ErrorArgs {
        use spark_core::{ErrorArg, ErrorArgs};
        use std::sync::Arc;
        match self {
            Self::CheckFailed { count } => ErrorArgs::new().with("count", ErrorArg::Unsigned(*count as u64)),
            Self::Internal { detail } => {
                // `detail` 仅供调试器展开，不得作为用户可见句子权威。
                ErrorArgs::new().with("opaque", ErrorArg::String(Arc::from(detail.as_str())))
            }
        }
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for CompileError {}

/// 编译选项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompileOptions {
    /// 检查失败时是否拒绝产出。
    pub reject_on_check_failure: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self { reject_on_check_failure: true }
    }
}

/// 编译结果（包 + 可选检查报告）。
#[derive(Debug, Clone)]
pub struct CompileOutput {
    pub bundle: LocalizationBundle,
    pub report: CheckReport,
}

/// 将一组 [`LocalizationDocument`] 编译为运行时包。
pub fn compile_documents(documents: &[LocalizationDocument], options: CompileOptions) -> Result<CompileOutput, CompileError> {
    let mut report = CheckReport::default();
    for doc in documents {
        report.issues.extend(check_document(doc).issues);
    }
    if options.reject_on_check_failure && !report.is_ok() {
        return Err(CompileError::check_failed(report.error_count()));
    }

    let mut bundle = LocalizationBundle::new();
    for doc in documents {
        for (name, def) in &doc.messages {
            bundle.insert(doc.namespace.clone(), name.clone(), doc.locale.clone(), CompiledMessage::from_definition(def));
        }
    }

    Ok(CompileOutput { bundle, report })
}

/// 便捷：编译单个文档。
pub fn compile_document(doc: &LocalizationDocument) -> Result<LocalizationBundle, CompileError> {
    Ok(compile_documents(std::slice::from_ref(doc), CompileOptions::default())?.bundle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{document::MessageDefinition, locale::LocaleId};
    use std::sync::Arc;

    #[test]
    fn compiles_text_and_template_sugar() {
        let mut doc = LocalizationDocument::new(LocaleId::parse("en").unwrap(), "game");
        doc.insert("plain", MessageDefinition::Text(Arc::from("Hi")));
        doc.insert("hello", MessageDefinition::Text(Arc::from("Hello, {name}")));
        let bundle = compile_document(&doc).unwrap();
        assert_eq!(bundle.len(), 2);
        assert!(matches!(bundle.get_named("game", "plain", &doc.locale), Some(CompiledMessage::Text(_))));
        assert!(matches!(bundle.get_named("game", "hello", &doc.locale), Some(CompiledMessage::Pattern(_))));
        assert_ne!(bundle.content_hash, 0);
    }
}
