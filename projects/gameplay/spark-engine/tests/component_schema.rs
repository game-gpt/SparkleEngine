//! 脚本组件列布局登记与校验。

use spark_engine::{
    ScriptComponentCatalog, ScriptComponentLayout, ScriptComponentLayoutBuilder, ScriptComponentLayoutError, ScriptFieldKind,
};

#[test]
fn layout_builder_aligns_fields() {
    let layout = ScriptComponentLayoutBuilder::new()
        .field("hp", ScriptFieldKind::F32)
        .field("alive", ScriptFieldKind::Bool)
        .build()
        .unwrap();
    assert_eq!(layout.size, 8);
    assert_eq!(layout.field("hp").unwrap().offset, 0);
    assert_eq!(layout.field("alive").unwrap().offset, 4);
}

#[test]
fn catalog_stores_layout_by_slot() {
    let layout = ScriptComponentLayoutBuilder::new().field("x", ScriptFieldKind::F32).build().unwrap();
    let mut catalog = ScriptComponentCatalog::with_builtins();
    let id = catalog.register_with_layout("Health", layout.clone());
    assert_eq!(catalog.layout_of(id), Some(&layout));
    assert_eq!(catalog.layout_of_name("Health"), Some(&layout));
}

#[test]
fn overlapping_fields_rejected() {
    let err = ScriptComponentLayout::new(
        vec![
            spark_engine::ScriptFieldLayout { name: "a".into(), kind: ScriptFieldKind::F32, offset: 0 },
            spark_engine::ScriptFieldLayout { name: "b".into(), kind: ScriptFieldKind::I32, offset: 0 },
        ],
        4,
    )
    .unwrap_err();
    assert!(matches!(err, ScriptComponentLayoutError::OverlappingFields { .. }));
}
