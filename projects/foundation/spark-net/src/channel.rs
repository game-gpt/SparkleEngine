//! 包头与通道。

/// 包序号（`u16` 环绕）。可靠通道用其检测乱序与重传窗口。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sequence(pub u16);

impl Sequence {
    /// 下一个序号（`wrapping_add(1)`，溢出回 0）。
    pub fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }

    /// 无符号环绕距离（后减前），结果为有符号 `i16`，用于比较先后。
    pub fn dist_from(self, earlier: Self) -> i16 {
        self.0.wrapping_sub(earlier.0) as i16
    }
}

/// 传输通道语义（由上层协议解释，本 crate 不强制实现）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    /// 可丢弃：适合状态快照；迟到包可直接丢弃。
    Unreliable,
    /// 有序可靠：适合命令流；要求送达且按序号处理。
    ReliableOrdered,
}

/// 网络包头：通道、序号与权威 tick。
#[derive(Debug, Clone, Copy)]
pub struct PacketHeader {
    /// 本包走哪条通道语义。
    pub channel: ChannelKind,
    /// 发送端递增的包序号。
    pub sequence: Sequence,
    /// 权威仿真 tick（客户端预测对齐用）；单位为离散帧计数，非墙钟。
    pub tick: u32,
}

/// 一帧网络载荷：头 + 原始字节（无游戏消息解码）。
#[derive(Debug, Clone)]
pub struct NetPacket {
    /// 路由与时序元数据。
    pub header: PacketHeader,
    /// 应用层字节；本 crate 不解释内容。
    pub payload: Vec<u8>,
}
