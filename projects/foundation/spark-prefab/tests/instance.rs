//! 覆盖路径与实例校验。

use spark_asset::MetaValue;
use spark_prefab::{PrefabDocument, PrefabError, PrefabInstance, parse_override_path};

#[test]
fn parse_override_path_ok() {
    let p = parse_override_path("player/body/Transform.position").unwrap();
    assert_eq!(p.node_path, "player/body");
    assert_eq!(p.component, "Transform");
    assert_eq!(p.field, "position");
    assert_eq!(p.to_key(), "player/body/Transform.position");
}

#[test]
fn instance_override_validated() {
    let mut doc = PrefabDocument::new("player");
    doc.ensure_child("player", "body").unwrap();
    doc.ensure_component("player", "Transform").unwrap();
    doc.ensure_component("body", "Transform").unwrap();
    doc.validate().unwrap();

    let mut inst = PrefabInstance::new("assets/player.prefab", "spawn");
    inst.set_override(
        "player/Transform.position",
        MetaValue::Array(vec![MetaValue::Int(1), MetaValue::Int(2)]),
    );
    inst.set_override(
        "player/body/Transform.position",
        MetaValue::Array(vec![MetaValue::Int(3), MetaValue::Int(4)]),
    );
    inst.validate_against(&doc).unwrap();

    inst.set_override("player/missing/Transform.x", MetaValue::Int(1));
    let err = inst.validate_against(&doc).unwrap_err();
    assert!(matches!(
        err,
        PrefabError::OverrideTargetMissing { .. } | PrefabError::BadOverridePath { .. }
    ));
}
