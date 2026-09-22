//! 自 `src/prediction.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_net::{ChannelKind, InMemoryBus, NetPacket, PacketHeader, PeerId, PredictionClock, Sequence, Transport};

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
                header: PacketHeader { channel: ChannelKind::ReliableOrdered, sequence: Sequence(1), tick: 3 },
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
