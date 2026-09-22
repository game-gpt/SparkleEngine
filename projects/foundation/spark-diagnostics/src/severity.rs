//! 诊断严重级别。

/// 诊断严重级别（升序：信息 → 缺陷）。
///
/// 渲染器可按级别着色 / 过滤；不影响 [`crate::Error`] 的相等性。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// 补充说明，不表示故障。
    Note,
    /// 修复建议或下一步操作提示。
    Help,
    /// 可继续执行的异常情况。
    Warning,
    /// 操作失败，调用方应中止或回退。
    Error,
    /// 引擎内部不变式被破坏，应视为缺陷。
    Bug,
}
