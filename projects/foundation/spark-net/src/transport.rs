//! 传输抽象与进程内环回。

use std::collections::{HashMap, VecDeque};

use crate::{NetError, channel::NetPacket};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PeerId(pub u32);

pub trait Transport {
    fn send(&mut self, to: PeerId, packet: NetPacket) -> Result<(), NetError>;
    fn recv(&mut self) -> Vec<(PeerId, NetPacket)>;
    fn poll(&mut self) {}
}

/// 单进程测试用环回总线（所有端共享同一 [`InMemoryBus`]）。
#[derive(Debug, Default)]
pub struct InMemoryBus {
    inbox: HashMap<PeerId, VecDeque<(PeerId, NetPacket)>>,
}

impl InMemoryBus {
    pub fn endpoint(&mut self, id: PeerId) -> InMemoryTransport<'_> {
        self.inbox.entry(id).or_default();
        InMemoryTransport { id, bus: self }
    }
}

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
