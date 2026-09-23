//! `ColumnDispatchTable` 绑定期分发表单元测。

use spark_engine::{
    ColumnDispatchTable, ScriptComponentLayoutBuilder, ScriptFieldKind, ScriptSystemDescriptor,
};
use spark_engine::command_apply::ScriptComponentCatalog;
use spark_script::HostPhase;

#[test]
fn bind_aligns_fields_with_query_plan_columns() {
    let mut catalog = ScriptComponentCatalog::with_builtins();
    let layout = ScriptComponentLayoutBuilder::new()
        .field("hp", ScriptFieldKind::F32)
        .field("alive", ScriptFieldKind::Bool)
        .build()
        .unwrap();
    catalog.register_with_layout("Health", layout);

    let desc = ScriptSystemDescriptor::new("m", "update", "update", HostPhase::Update)
        .write("Health")
        .query_archetype("rock");
    let plan = spark_engine::QueryPlan::bind(&desc, &catalog).unwrap();
    let dispatch = ColumnDispatchTable::bind(&plan, &catalog);

    assert_eq!(dispatch.column_count(), plan.columns().len());
    let field = dispatch.field(0, 0).expect("hp field");
    let slot = catalog.id_of("Health").unwrap();
    assert_eq!(field.slot, slot);
    assert_eq!(field.kind, ScriptFieldKind::F32);
    assert!(field.column_write);
    let alive = dispatch.field(0, 1).expect("alive field");
    assert_eq!(alive.kind, ScriptFieldKind::Bool);
}

#[test]
fn read_write_dispatch_roundtrip_without_layout_lookup() {
    use spark_engine::{ColumnFieldValue, ScriptComponentStore};

    let mut catalog = ScriptComponentCatalog::with_builtins();
    let layout = ScriptComponentLayoutBuilder::new()
        .field("score", ScriptFieldKind::I32)
        .build()
        .unwrap();
    catalog.register_with_layout("Stats", layout.clone());

    let desc = ScriptSystemDescriptor::new("m", "update", "update", HostPhase::Update)
        .write("Stats")
        .query_archetype("rock");
    let plan = spark_engine::QueryPlan::bind(&desc, &catalog).unwrap();
    let dispatch = ColumnDispatchTable::bind(&plan, &catalog);
    let field = dispatch.field(0, 0).unwrap();

    let mut store = ScriptComponentStore::new();
    let slot = catalog.id_of("Stats").unwrap();
    store.ensure_row(slot, 42, &layout);
    assert!(store.write_dispatch(field, 42, ColumnFieldValue::I32(-3)));
    let read = store.read_dispatch(field, 42).unwrap();
    assert_eq!(read, ColumnFieldValue::I32(-3));
}
