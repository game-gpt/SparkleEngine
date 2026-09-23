//! ScriptComponentStore 多类型字段读写。

use spark_engine::{ScriptComponentLayoutBuilder, ScriptComponentStore, ScriptFieldKind};

#[test]
fn store_reads_and_writes_i32_and_bool_fields() {
    let mut store = ScriptComponentStore::new();
    let layout = ScriptComponentLayoutBuilder::new()
        .field("score", ScriptFieldKind::I32)
        .field("alive", ScriptFieldKind::Bool)
        .build()
        .unwrap();
    let slot = spark_engine::ComponentDescriptorId(0);
    store.ensure_row(slot, 42, &layout);
    assert!(store.write_i32(slot, &layout, 42, 0, -3));
    assert!(store.write_bool(slot, &layout, 42, 1, true));
    assert_eq!(store.read_i32(slot, &layout, 42, 0), Some(-3));
    assert_eq!(store.read_bool(slot, &layout, 42, 1), Some(true));
    assert!(store.write_bool(slot, &layout, 42, 1, false));
    assert_eq!(store.read_bool(slot, &layout, 42, 1), Some(false));
}
