# spark-gc

Mark–sweep heap for scripts.

```rust
use spark_gc::Heap;

let mut heap = Heap::new();
let keep = heap.alloc_string("keep");
let _drop = heap.alloc_string("drop");
assert_eq!(heap.live_count(), 2);
heap.collect(&[keep]);
assert_eq!(heap.live_count(), 1);
```

`Value::Entity` / `Value::Func` immediates are not heap-allocated. Errors: `GcError`. Roots are registered by hosts such as `spark-vm`.

```bash
cargo test -p spark-gc
```
