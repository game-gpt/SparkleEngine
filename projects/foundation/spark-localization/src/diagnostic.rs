//! 消息求值诊断。

use std::{fmt, sync::Arc};

use crate::{locale::LocaleId, message::MessageRef};

/// 单次格式化可能附带的诊断类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageDiagnostic {
    /// 回退链上仍找不到消息键。
    Missing,
    /// 参数类型与节点期望不匹配。
    BadArgument,
    /// 节点需要的参数未传入。
    MissingArgument,
    /// 传入了消息未声明的参数（预留；当前求值路径未必置位）。
    UnknownArgument,
    /// 消息引用求值检测到环。
    Cycle,
    /// 富文本 / 标记不在白名单内（预留）。
    InvalidMarkup,
    /// 关联资源键无效（预留）。
    InvalidResource,
    /// 未命中请求 Locale，改用了回退链上的其它 Locale。
    FallbackUsed,
}

impl fmt::Display for MessageDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Missing => "missing",
            Self::BadArgument => "bad-argument",
            Self::MissingArgument => "missing-argument",
            Self::UnknownArgument => "unknown-argument",
            Self::Cycle => "cycle",
            Self::InvalidMarkup => "invalid-markup",
            Self::InvalidResource => "invalid-resource",
            Self::FallbackUsed => "fallback-used",
        };
        f.write_str(name)
    }
}

/// 紧凑诊断位标记，随 [`crate::LocalizedText`] 返回。
///
/// 各位与 [`MessageDiagnostic`] 一一对应；可用按位并集组合多次求值结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DiagnosticFlags(pub u32);

impl DiagnosticFlags {
    /// 无诊断。
    pub const NONE: Self = Self(0);
    /// 对应 [`MessageDiagnostic::Missing`]。
    pub const MISSING: Self = Self(1 << 0);
    /// 对应 [`MessageDiagnostic::BadArgument`]。
    pub const BAD_ARGUMENT: Self = Self(1 << 1);
    /// 对应 [`MessageDiagnostic::MissingArgument`]。
    pub const MISSING_ARGUMENT: Self = Self(1 << 2);
    /// 对应 [`MessageDiagnostic::UnknownArgument`]。
    pub const UNKNOWN_ARGUMENT: Self = Self(1 << 3);
    /// 对应 [`MessageDiagnostic::Cycle`]。
    pub const CYCLE: Self = Self(1 << 4);
    /// 对应 [`MessageDiagnostic::InvalidMarkup`]。
    pub const INVALID_MARKUP: Self = Self(1 << 5);
    /// 对应 [`MessageDiagnostic::InvalidResource`]。
    pub const INVALID_RESOURCE: Self = Self(1 << 6);
    /// 对应 [`MessageDiagnostic::FallbackUsed`]。
    pub const FALLBACK_USED: Self = Self(1 << 7);

    /// 空标志，等同 [`Self::NONE`]。
    pub const fn empty() -> Self {
        Self::NONE
    }

    /// 原始位图。
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// 是否包含 `other` 的全部置位。
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// 是否与 `other` 有任意共同置位。
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// 按位或合并。
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// 就地并入另一组标志。
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// 是否全零。
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// 由单一诊断类别构造对应位。
    pub const fn from_diagnostic(kind: MessageDiagnostic) -> Self {
        match kind {
            MessageDiagnostic::Missing => Self::MISSING,
            MessageDiagnostic::BadArgument => Self::BAD_ARGUMENT,
            MessageDiagnostic::MissingArgument => Self::MISSING_ARGUMENT,
            MessageDiagnostic::UnknownArgument => Self::UNKNOWN_ARGUMENT,
            MessageDiagnostic::Cycle => Self::CYCLE,
            MessageDiagnostic::InvalidMarkup => Self::INVALID_MARKUP,
            MessageDiagnostic::InvalidResource => Self::INVALID_RESOURCE,
            MessageDiagnostic::FallbackUsed => Self::FALLBACK_USED,
        }
    }
}

impl From<MessageDiagnostic> for DiagnosticFlags {
    fn from(value: MessageDiagnostic) -> Self {
        Self::from_diagnostic(value)
    }
}

/// 结构化诊断记录（开发期日志 / 覆盖率工具）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticRecord {
    /// 诊断类别。
    pub kind: MessageDiagnostic,
    /// 出问题的消息引用。
    pub message: MessageRef,
    /// 调用方请求的 Locale。
    pub requested: LocaleId,
    /// 实际命中的 Locale；完全缺失时为 `None`。
    pub resolved: Option<LocaleId>,
    /// 可选机器可读细节（参数名等），非用户句子。
    pub detail: Option<Arc<str>>,
}
