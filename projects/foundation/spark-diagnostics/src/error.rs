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
#[derive(Clone)]
pub struct Error {
    pub code: ErrorCode,
    pub args: ErrorArgs,
    pub context: ErrorContext,
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
    pub fn new(code: ErrorCode) -> Self {
        Self { code, args: ErrorArgs::new(), context: ErrorContext::new(), cause: None }
    }

    pub fn with_args(mut self, args: ErrorArgs) -> Self {
        self.args = args;
        self
    }

    pub fn with_context(mut self, context: ErrorContext) -> Self {
        self.context = context;
        self
    }

    pub fn caused_by(mut self, cause: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.cause = Some(Arc::new(cause));
        self
    }

    pub fn arg(mut self, name: impl Into<Arc<str>>, value: crate::ErrorArg) -> Self {
        self.args.insert(name, value);
        self
    }

    pub fn not_implemented(what: &'static str) -> Self {
        Self::new(codes::not_implemented()).arg("what", crate::ErrorArg::String(Arc::from(what)))
    }

    pub fn invalid_argument(name: impl AsRef<str>) -> Self {
        Self::new(codes::invalid_argument()).arg("name", crate::ErrorArg::String(Arc::from(name.as_ref())))
    }

    pub fn invalid_state(detail: impl AsRef<str>) -> Self {
        Self::new(codes::invalid_state()).arg("detail", crate::ErrorArg::String(Arc::from(detail.as_ref())))
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_stable_code_only() {
        let err = Error::not_implemented("widget-rtl");
        assert_eq!(err.to_string(), "spark.not_implemented");
        assert!(err.args.get("what").is_some());
    }
}
