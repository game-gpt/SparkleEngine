//! Spark **网络框架**（无游戏消息语义）。
//!
//! 提供不可靠/可靠通道抽象、包序号、简易客户端预测与权威确认。
//! **不**定义游戏 RPC 枚举或具体传输实现（UDP/WebRTC 由宿主注入）。

mod channel;
mod prediction;
mod transport;

pub use channel::{ChannelKind, NetPacket, PacketHeader, Sequence};
pub use prediction::{PredictionClock, PredictionError};
pub use transport::{InMemoryTransport, PeerId, Transport};

use spark_core::SparkError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetError {
    #[error(transparent)]
    Spark(#[from] SparkError),
    #[error("{0}")]
    Message(String),
    #[error("对端未连接：{0:?}")]
    NotConnected(PeerId),
}
