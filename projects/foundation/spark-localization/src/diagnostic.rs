//! 消息求值诊断。

use std::{fmt, sync::Arc};

use crate::{locale::LocaleId, message::MessageRef};

/// 单次格式化可能附带的诊断类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageDiagnostic {
    Missing,
    BadArgument,
    MissingArgument,
    UnknownArgument,
    Cycle,
    InvalidMarkup,
    InvalidResource,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DiagnosticFlags(pub u32);

impl DiagnosticFlags {
    pub const NONE: Self = Self(0);
    pub const MISSING: Self = Self(1 << 0);
    pub const BAD_ARGUMENT: Self = Self(1 << 1);
    pub const MISSING_ARGUMENT: Self = Self(1 << 2);
    pub const UNKNOWN_ARGUMENT: Self = Self(1 << 3);
    pub const CYCLE: Self = Self(1 << 4);
    pub const INVALID_MARKUP: Self = Self(1 << 5);
    pub const INVALID_RESOURCE: Self = Self(1 << 6);
    pub const FALLBACK_USED: Self = Self(1 << 7);

    pub const fn empty() -> Self {
        Self::NONE
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

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
    pub kind: MessageDiagnostic,
    pub message: MessageRef,
    pub requested: LocaleId,
    pub resolved: Option<LocaleId>,
    pub detail: Option<Arc<str>>,
}
