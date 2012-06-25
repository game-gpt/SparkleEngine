//! 错误上下文（调用点、目标等元数据）。

use std::sync::Arc;

/// 附加在错误上的非参数元数据。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorContext {
    pub target: Option<Arc<str>>,
    pub operation: Option<Arc<str>>,
    pub file: Option<Arc<str>>,
    pub line: Option<u32>,
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
}
