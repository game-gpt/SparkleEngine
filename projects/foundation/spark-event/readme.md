# spark-event

Typed double-buffered event bus `EventBus`.

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

`send` writes the writing buffer; after `update_all` swaps buffers, `events::<E>()` iterates the previous frame. Multiple event types can coexist. Event enums are defined by callers.

```bash
cargo test -p spark-event
```
