# spark-circuit

导体图：节点、通道（如 `Channel::CONTROL` / `POWER`）与可达查询。

```rust
use spark_circuit::{Channel, CircuitGraph};

let mut g = CircuitGraph::new();
let a = g.add_node();
let b = g.add_node();
let c = g.add_node();
g.link(a, b, Channel::CONTROL).unwrap();
g.link(b, c, Channel::CONTROL).unwrap();
assert!(g.can_reach(a, c, Channel::CONTROL).unwrap());

g.unlink(b, c, Channel::CONTROL).unwrap();
assert!(!g.can_reach(a, c, Channel::CONTROL).unwrap());

let mask = g.reachable_from_any(&[a], Channel::CONTROL).unwrap();
```

错误：`CircuitError`。另有连通分量、功率预算等 API。

```bash
cargo test -p spark-circuit
```
