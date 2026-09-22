//! 错误上下文（调用点、目标、源码范围等元数据）。

use std::sync::Arc;

use crate::diagnostic::SourceSpan;

/// 附加在错误上的非参数元数据。
///
/// 与 [`crate::ErrorArgs`] 分工：参数参与本地化插值；上下文供日志 / IDE 定位，
/// 一般不进用户可见主句。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorContext {
    /// 操作目标标识（资源键、实体调试名等）。
    pub target: Option<Arc<str>>,
    /// 正在执行的操作名（如 `load`、`upload`）。
    pub operation: Option<Arc<str>>,
    /// 源文件逻辑路径（脚本 / 配置，非本机盘符权威）。
    pub file: Option<Arc<str>>,
    /// 与 `file` 对应的 1-based 行号；无文件时忽略。
    pub line: Option<u32>,
    /// 源码字节范围（脚本 / 资产文本）。
    pub span: Option<SourceSpan>,
}

impl ErrorContext {
    /// 全空上下文。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置操作目标。
    pub fn target(mut self, target: impl Into<Arc<str>>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// 设置操作名。
    pub fn operation(mut self, operation: impl Into<Arc<str>>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    /// 绑定源文件与行号（行号为 1-based）。
    pub fn at(mut self, file: impl Into<Arc<str>>, line: u32) -> Self {
        self.file = Some(file.into());
        self.line = Some(line);
        self
    }

    /// 绑定字节级源码范围。
    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }
}
