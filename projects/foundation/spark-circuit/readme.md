# spark-circuit

Conductor graph: nodes, channels (e.g. `Channel::CONTROL` / `POWER`), and reachability queries.

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

Errors: `CircuitError`. Also connected components, power budgets, and more.

```bash
cargo test -p spark-circuit
```
