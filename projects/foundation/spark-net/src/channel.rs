//! 包头与通道。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sequence(pub u16);

impl Sequence {
    pub fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }

    /// 无符号环绕距离（后减前）。
    pub fn dist_from(self, earlier: Self) -> i16 {
        self.0.wrapping_sub(earlier.0) as i16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    /// 可丢弃（状态快照）。
    Unreliable,
    /// 有序可靠（命令）。
    ReliableOrdered,
}

#[derive(Debug, Clone, Copy)]
pub struct PacketHeader {
    pub channel: ChannelKind,
    pub sequence: Sequence,
    /// 权威 tick（客户端预测对齐用）。
    pub tick: u32,
}

#[derive(Debug, Clone)]
pub struct NetPacket {
    pub header: PacketHeader,
    pub payload: Vec<u8>,
}
