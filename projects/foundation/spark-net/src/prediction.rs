//! 客户端预测时钟。

use thiserror::Error;

use crate::channel::Sequence;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PredictionError {
    #[error("权威 tick 回退")]
    TickRewind,
    #[error("输入缓冲溢出")]
    BufferOverflow,
}

/// 本地预测 tick 与待确认输入序号。
#[derive(Debug, Clone)]
pub struct PredictionClock {
    pub local_tick: u32,
    pub last_acked_tick: u32,
    pub next_input_seq: Sequence,
    pending: Vec<(u32, Sequence)>,
    cap: usize,
}

impl PredictionClock {
    pub fn new(buffer_cap: usize) -> Self {
        Self {
            local_tick: 0,
            last_acked_tick: 0,
            next_input_seq: Sequence(0),
            pending: Vec::new(),
            cap: buffer_cap.max(1),
        }
    }

    /// 推进本地 tick 并登记一条输入序号。
    pub fn push_input(&mut self) -> Result<Sequence, PredictionError> {
        if self.pending.len() >= self.cap {
            return Err(PredictionError::BufferOverflow);
        }
        let seq = self.next_input_seq;
        self.next_input_seq = seq.next();
        self.local_tick = self.local_tick.wrapping_add(1);
        self.pending.push((self.local_tick, seq));
        Ok(seq)
    }

    /// 权威确认到 `tick`（含），丢弃更早的预测输入。
    pub fn acknowledge(&mut self, tick: u32) -> Result<usize, PredictionError> {
        if tick < self.last_acked_tick {
            return Err(PredictionError::TickRewind);
        }
        self.last_acked_tick = tick;
        let before = self.pending.len();
        self.pending.retain(|(t, _)| *t > tick);
        Ok(before - self.pending.len())
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::{ChannelKind, NetPacket, PacketHeader};
    use crate::transport::{InMemoryBus, PeerId, Transport};

    #[test]
    fn loopback_and_ack() {
        let mut bus = InMemoryBus::default();
        let a = PeerId(1);
        let b = PeerId(2);
        {
            let mut ea = bus.endpoint(a);
            ea.send(
                b,
                NetPacket {
                    header: PacketHeader {
                        channel: ChannelKind::ReliableOrdered,
                        sequence: Sequence(1),
                        tick: 3,
                    },
                    payload: b"hi".to_vec(),
                },
            )
            .unwrap();
        }
        let mut eb = bus.endpoint(b);
        let got = eb.recv();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0, a);
        assert_eq!(got[0].1.payload, b"hi");

        let mut clock = PredictionClock::new(8);
        let _ = clock.push_input().unwrap();
        let _ = clock.push_input().unwrap();
        assert_eq!(clock.acknowledge(1).unwrap(), 1);
        assert_eq!(clock.pending_len(), 1);
    }
}
