//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_gc::*;

#[test]
fn collect_unreachable_string() {
    let mut heap = Heap::new();
    let keep = heap.alloc_string("keep");
    let _drop = heap.alloc_string("drop");
    assert_eq!(heap.live_count(), 2);
    heap.collect(&[keep]);
    assert_eq!(heap.live_count(), 1);
}

#[test]
fn entity_and_func_are_not_heap() {
    let e = Value::Entity(7);
    let f = Value::Func(3);
    assert_eq!(e.as_entity(), Some(7));
    assert_eq!(f.as_func(), Some(3));
    assert!(e.truthy());
}
