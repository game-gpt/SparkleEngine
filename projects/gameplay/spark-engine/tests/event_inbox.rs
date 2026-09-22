//! 自 `src/event_inbox.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine::*;

use spark_gc::Value;
#[test]
fn drain_preserves_order() {
    let mut inbox = ScriptEventInbox::new();
    inbox.push("a", vec![Value::Number(1.0)]);
    inbox.push("b", vec![Value::Number(2.0)]);
    let ev = inbox.drain();
    assert_eq!(ev.len(), 2);
    assert_eq!(ev[0].name.as_ref(), "a");
    assert_eq!(ev[1].name.as_ref(), "b");
    assert!(inbox.is_empty());
}
