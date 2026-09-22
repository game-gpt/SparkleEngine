# spark-net

网络传输抽象、包序号与简易客户端预测。

```rust
use spark_net::{
    ChannelKind, InMemoryBus, NetPacket, PacketHeader, PeerId, PredictionClock, Sequence, Transport,
};

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
let got = bus.endpoint(b).recv();
assert_eq!(got[0].1.payload, b"hi");

let mut clock = PredictionClock::new(8);
let _ = clock.push_input().unwrap();
```

具体 UDP/WebRTC 由宿主实现 `Transport`。错误：`NetError`、`PredictionError`。

```bash
cargo test -p spark-net
```
