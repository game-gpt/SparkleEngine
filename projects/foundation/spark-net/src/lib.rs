//! Spark **网络框架**（无游戏消息语义）。
//!
//! 提供不可靠/可靠通道抽象、包序号、简易客户端预测与权威确认。
//! **不**定义游戏 RPC 枚举或具体传输实现（UDP/WebRTC 由宿主注入）。

#![warn(missing_docs)]
mod channel;
mod prediction;
mod transport;

pub use channel::{ChannelKind, NetPacket, PacketHeader, Sequence};
pub use prediction::{PredictionClock, PredictionError};
pub use transport::{InMemoryBus, InMemoryTransport, PeerId, Transport};

use std::{fmt, sync::Arc};

use spark_types::{ErrorArg, ErrorArgs, SparkError};

/// 网络层结构化错误。`Display` 只输出稳定码。
#[derive(Debug)]
pub enum NetError {
    Spark(SparkError),
    NotConnected(PeerId),
    /// `detail` 必须是机器令牌，不是自然语言。
    Internal {
        detail: String,
    },
}

impl NetError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Spark(_) => "spark.net.spark",
            Self::NotConnected(_) => "spark.net.not_connected",
            Self::Internal { .. } => "spark.net.internal",
        }
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        Self::Internal { detail: detail.into() }
    }

    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::Spark(e) => e.args.clone(),
            Self::NotConnected(peer) => ErrorArgs::new().with("peer", ErrorArg::Unsigned(u64::from(peer.0))),
            Self::Internal { detail } => ErrorArgs::new().with("reason", ErrorArg::String(Arc::from(detail.as_str()))),
        }
    }
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spark(e) => e.fmt(f),
            other => f.write_str(other.code()),
        }
    }
}

impl std::error::Error for NetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spark(e) => Some(e),
            _ => None,
        }
    }
}

impl From<SparkError> for NetError {
    fn from(value: SparkError) -> Self {
        Self::Spark(value)
    }
}
