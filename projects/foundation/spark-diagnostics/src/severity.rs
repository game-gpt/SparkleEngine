//! 诊断严重级别。

/// 诊断严重级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Note,
    Help,
    Warning,
    Error,
    Bug,
}
