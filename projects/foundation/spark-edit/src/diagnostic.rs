//! 机器可读诊断。

use serde::Serialize;

/// 诊断严重级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// 错误（阻断 apply）。
    Error,
    /// 警告。
    Warning,
    /// 信息。
    Info,
}

/// 单条诊断。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// 级别。
    pub severity: Severity,
    /// 稳定错误码。
    pub code: String,
    /// 简短说明（English/中文皆可；权威仍是 `code`）。
    pub message: String,
    /// 相关路径（若有）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}
