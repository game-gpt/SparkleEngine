# spark-event

类型化双缓冲事件总线 `EventBus`。

```rust
use spark_event::EventBus;

#[derive(Clone, Debug, PartialEq)]
struct Boom(u32);

let mut bus = EventBus::new();
bus.send(Boom(1));
bus.send(Boom(2));
bus.update_all();
let got: Vec<_> = bus.events::<Boom>().unwrap().iter().map(|b| b.0).collect();
assert_eq!(got, vec![1, 2]);
```

`send` 写入 writing 缓冲；`update_all` 交换缓冲后，`events::<E>()` 迭代上一拍。可并存多种事件类型。事件枚举由调用方定义。

```bash
cargo test -p spark-event
```
