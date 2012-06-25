//! 错误命名空间与稳定错误码。

use std::fmt;
use std::sync::Arc;

/// 错误码命名空间（如 `spark`、`spark.asset`）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamespaceId(Arc<str>);

impl NamespaceId {
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NamespaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 命名空间内错误标识（点分，如 `not_found`）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErrorId(Arc<str>);

impl ErrorId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ErrorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 稳定错误码：`namespace` + `id` → 显示为 `spark.asset.not_found`。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErrorCode {
    pub namespace: NamespaceId,
    pub id: ErrorId,
}

impl ErrorCode {
    pub fn new(namespace: impl Into<Arc<str>>, id: impl Into<Arc<str>>) -> Self {
        Self {
            namespace: NamespaceId::new(namespace),
            id: ErrorId::new(id),
        }
    }

    /// 解析 `spark.asset.not_found`（首段为命名空间，其余为 id）。
    pub fn parse(dotted: &str) -> Self {
        if let Some((ns, rest)) = dotted.split_once('.') {
            Self::new(ns, rest)
        } else {
            Self::new("spark", dotted)
        }
    }

    pub fn as_dotted(&self) -> String {
        format!("{}.{}", self.namespace, self.id)
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.namespace, self.id)
    }
}

/// 引擎保留码（最小集；领域 crate 可另定义常量）。
pub mod codes {
    use super::ErrorCode;

    pub fn not_implemented() -> ErrorCode {
        ErrorCode::new("spark", "not_implemented")
    }

    pub fn invalid_argument() -> ErrorCode {
        ErrorCode::new("spark", "invalid_argument")
    }

    pub fn invalid_state() -> ErrorCode {
        ErrorCode::new("spark", "invalid_state")
    }

    pub fn unsupported() -> ErrorCode {
        ErrorCode::new("spark", "unsupported")
    }

    pub fn resource_unavailable() -> ErrorCode {
        ErrorCode::new("spark", "resource_unavailable")
    }

    pub fn internal_invariant() -> ErrorCode {
        ErrorCode::new("spark", "internal_invariant")
    }

    pub fn io() -> ErrorCode {
        ErrorCode::new("spark", "io")
    }

    pub fn asset_not_found() -> ErrorCode {
        ErrorCode::new("spark", "asset.not_found")
    }

    pub fn asset_io() -> ErrorCode {
        ErrorCode::new("spark", "asset.io")
    }

    pub fn localization_locale_invalid() -> ErrorCode {
        ErrorCode::new("spark", "localization.locale_invalid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotted_roundtrip() {
        let c = ErrorCode::parse("spark.asset.not_found");
        assert_eq!(c.namespace.as_str(), "spark");
        assert_eq!(c.id.as_str(), "asset.not_found");
        assert_eq!(c.to_string(), "spark.asset.not_found");
    }
}
