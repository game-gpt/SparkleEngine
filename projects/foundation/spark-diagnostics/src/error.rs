//! 结构化错误值。

use std::{fmt, sync::Arc};

use crate::{
    args::ErrorArgs,
    code::{ErrorCode, codes},
    context::ErrorContext,
};

/// 因果：标准库错误对象（第三方 opaque 等）。
pub type ErrorCause = Arc<dyn std::error::Error + Send + Sync>;

/// 结构化错误事实。
///
/// 只保存码、类型化参数、上下文与因果；最终用户句子由 `spark-localization`
/// 在渲染边界生成。[`Display`] 仅输出稳定错误码字符串。
#[derive(Clone)]
pub struct Error {
    /// 稳定错误码（权威身份）。
    pub code: ErrorCode,
    /// 本地化插值用的类型化参数。
    pub args: ErrorArgs,
    /// 调用点 / 目标等非插值元数据。
    pub context: ErrorContext,
    /// 下层标准错误（opaque）；不参与 [`PartialEq`]。
    pub cause: Option<ErrorCause>,
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("code", &self.code)
            .field("args", &self.args)
            .field("context", &self.context)
            .field("cause", &self.cause.as_ref().map(|c| c.to_string()))
            .finish()
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code && self.args == other.args && self.context == other.context
        // cause 不参与相等（动态类型）。
    }
}

impl Error {
    /// 仅带错误码的空参数错误。
    pub fn new(code: ErrorCode) -> Self {
        Self { code, args: ErrorArgs::new(), context: ErrorContext::new(), cause: None }
    }

    /// 替换整表参数。
    pub fn with_args(mut self, args: ErrorArgs) -> Self {
        self.args = args;
        self
    }

    /// 替换上下文。
    pub fn with_context(mut self, context: ErrorContext) -> Self {
        self.context = context;
        self
    }

    /// 挂接下层 `std::error::Error` 作为因果链。
    pub fn caused_by(mut self, cause: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.cause = Some(Arc::new(cause));
        self
    }

    /// 追加单个具名参数。
    pub fn arg(mut self, name: impl Into<Arc<str>>, value: crate::ErrorArg) -> Self {
        self.args.insert(name, value);
        self
    }

    /// 快捷构造：`spark.not_implemented`，参数 `what` 为未实现能力的符号名。
    pub fn not_implemented(what: &'static str) -> Self {
        Self::new(codes::not_implemented()).arg("what", crate::ErrorArg::String(Arc::from(what)))
    }

    /// 快捷构造：`spark.invalid_argument`，参数 `name` 为非法参数名。
    pub fn invalid_argument(name: impl AsRef<str>) -> Self {
        Self::new(codes::invalid_argument()).arg("name", crate::ErrorArg::String(Arc::from(name.as_ref())))
    }

    /// 快捷构造：`spark.invalid_state`，参数 `detail` 为状态描述符号（非用户句）。
    pub fn invalid_state(detail: impl AsRef<str>) -> Self {
        Self::new(codes::invalid_state()).arg("detail", crate::ErrorArg::String(Arc::from(detail.as_ref())))
    }

    /// 快捷构造：`spark.internal_invariant`，参数 `detail` 为被破坏的不变式摘要。
    pub fn internal(detail: impl AsRef<str>) -> Self {
        Self::new(codes::internal_invariant()).arg("detail", crate::ErrorArg::String(Arc::from(detail.as_ref())))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 仅稳定码；自然语言由 localization / diagnostics 渲染器负责。
        f.write_str(&self.code.to_string())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause.as_ref().map(|c| c.as_ref() as &(dyn std::error::Error + 'static))
    }
}
