//! 传输抽象与进程内环回。

use std::collections::{HashMap, VecDeque};

use crate::{NetError, channel::NetPacket};

/// 对端标识（进程内或会话内唯一的 `u32`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PeerId(pub u32);

/// 字节包收发抽象；具体 UDP/WebRTC 由宿主实现并注入。
pub trait Transport {
    /// 向 `to` 发送一包；失败时返回 [`NetError`]（如未连接）。
    fn send(&mut self, to: PeerId, packet: NetPacket) -> Result<(), NetError>;
    /// 取出当前收件箱中全部包；每项为 `(发送方, 包)`。调用后收件箱清空。
    fn recv(&mut self) -> Vec<(PeerId, NetPacket)>;
    /// 轮询底层（驱动 IO、超时等）；默认空操作。
    fn poll(&mut self) {}
}

/// 单进程测试用环回总线（所有端共享同一 [`InMemoryBus`]）。
#[derive(Debug, Default)]
pub struct InMemoryBus {
    inbox: HashMap<PeerId, VecDeque<(PeerId, NetPacket)>>,
}

impl InMemoryBus {
    /// 取得（或登记）`id` 对应的传输端点；发往该 ID 的包进入其收件箱。
    pub fn endpoint(&mut self, id: PeerId) -> InMemoryTransport<'_> {
        self.inbox.entry(id).or_default();
        InMemoryTransport { id, bus: self }
    }
}

/// 挂在 [`InMemoryBus`] 上的一端；生命周期与总线借用绑定。
pub struct InMemoryTransport<'a> {
    id: PeerId,
    bus: &'a mut InMemoryBus,
}

impl Transport for InMemoryTransport<'_> {
    fn send(&mut self, to: PeerId, packet: NetPacket) -> Result<(), NetError> {
        self.bus.inbox.entry(to).or_default().push_back((self.id, packet));
        Ok(())
    }

    fn recv(&mut self) -> Vec<(PeerId, NetPacket)> {
        self.bus.inbox.get_mut(&self.id).map(|q| q.drain(..).collect()).unwrap_or_default()
    }
}
