//! 错误上下文（调用点、目标、源码范围等元数据）。

use std::sync::Arc;

use crate::diagnostic::SourceSpan;

/// 附加在错误上的非参数元数据。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorContext {
    pub target: Option<Arc<str>>,
    pub operation: Option<Arc<str>>,
    pub file: Option<Arc<str>>,
    pub line: Option<u32>,
    /// 源码字节范围（脚本 / 资产文本）。
    pub span: Option<SourceSpan>,
}

impl ErrorContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn target(mut self, target: impl Into<Arc<str>>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn operation(mut self, operation: impl Into<Arc<str>>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    pub fn at(mut self, file: impl Into<Arc<str>>, line: u32) -> Self {
        self.file = Some(file.into());
        self.line = Some(line);
        self
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }
}
