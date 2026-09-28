# spark-gc

脚本标记–清扫堆。

```rust
use spark_gc::Heap;

let mut heap = Heap::new();
let keep = heap.alloc_string("keep");
let _drop = heap.alloc_string("drop");
assert_eq!(heap.live_count(), 2);
heap.collect(&[keep]);
assert_eq!(heap.live_count(), 1);
```

`Value::Entity` / `Value::Func` 等立即数不进堆。错误：`GcError`。由 `spark-vm` 等宿主登记根。

```bash
cargo test -p spark-gc
```
